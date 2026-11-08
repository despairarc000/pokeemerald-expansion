//! Publish native map exports and project backups to user-accessible Downloads.
//! Android 10+ MediaStore requires no broad storage permissions.
use jni::{objects::{JObject, JValue, JString}, JNIEnv, JavaVM};
use std::{path::Path, sync::OnceLock};

static ANDROID_APP: OnceLock<winit::platform::android::activity::AndroidApp> = OnceLock::new();

pub fn set_app(app: winit::platform::android::activity::AndroidApp) {
    let _ = ANDROID_APP.set(app);
}

/// Real Android status/gesture/cutout insets in egui logical points.
/// The egui Android backend sometimes reports zero safe_area_insets even
/// while Android 15 draws behind system bars. Use native WindowInsets and
/// let the UI subtract whichever portion egui already accounted for.
pub fn safe_insets_logical(pixels_per_point:f32)->Option<(f32,f32)>{
    let app=ANDROID_APP.get()?;
    let vm=unsafe {JavaVM::from_raw(app.vm_as_ptr().cast())}.ok()?;
    let mut env=vm.attach_current_thread().ok()?;
    let activity=unsafe {JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject)};
    let result=(|| -> jni::errors::Result<(i32,i32)> {
        let window=env.call_method(&activity,"getWindow",
            "()Landroid/view/Window;",&[])?.l()?;
        let decor=env.call_method(&window,"getDecorView",
            "()Landroid/view/View;",&[])?.l()?;
        let insets=env.call_method(&decor,"getRootWindowInsets",
            "()Landroid/view/WindowInsets;",&[])?.l()?;
        if insets.is_null() {
            return Err(jni::errors::Error::NullPtr("RootWindowInsets not ready"));
        }
        let sdk=env.get_static_field("android/os/Build$VERSION","SDK_INT","I")?.i()?;
        if sdk>=30 {
            let bar=env.call_static_method("android/view/WindowInsets$Type",
                "systemBars","()I",&[])?.i()?;
            let cutout=env.call_static_method("android/view/WindowInsets$Type",
                "displayCutout","()I",&[])?.i()?;
            let gesture=env.call_static_method("android/view/WindowInsets$Type",
                "systemGestures","()I",&[])?.i()?;
            let types=bar | cutout | gesture;
            let rect=env.call_method(&insets,"getInsets",
                "(I)Landroid/graphics/Insets;",&[JValue::Int(types)])?.l()?;
            let top=env.get_field(&rect,"top","I")?.i()?;
            let bottom=env.get_field(&rect,"bottom","I")?.i()?;
            Ok((top,bottom))
        }else{
            let top=env.call_method(&insets,"getSystemWindowInsetTop","()I",&[])?.i()?;
            let bottom=env.call_method(&insets,"getSystemWindowInsetBottom","()I",&[])?.i()?;
            Ok((top,bottom))
        }
    })();
    let Ok((top,bottom))=result else{
        let _=env.exception_clear();
        return None;
    };
    let scale=pixels_per_point.max(1.0);
    Some((top.max(0) as f32/scale,bottom.max(0) as f32/scale))
}

/// Explicit, user-initiated native clipboard read for egui's Android forms.
/// The clipboard is NEVER read automatically or written to app storage.
/// Android 12+ may display its standard "pasted from clipboard" notice.
pub fn read_clipboard_text()->Result<String,String>{
    let app=ANDROID_APP.get().ok_or("Android Activity unavailable")?;
    let vm=unsafe {JavaVM::from_raw(app.vm_as_ptr().cast())}
        .map_err(|_|"Android clipboard Java VM unavailable")?;
    let mut env=vm.attach_current_thread()
        .map_err(|_|"Cannot access Android clipboard thread")?;
    // AndroidApp owns the Activity reference, as for MediaStore and insets.
    let activity=unsafe {JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject)};
    let result=(||->jni::errors::Result<Option<String>>{
        let name=JObject::from(env.new_string("clipboard")?);
        let manager=env.call_method(&activity,"getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&name)])?.l()?;
        if manager.is_null(){return Ok(None);}
        let clip=env.call_method(&manager,"getPrimaryClip",
            "()Landroid/content/ClipData;",&[])?.l()?;
        if clip.is_null(){return Ok(None);}
        let count=env.call_method(&clip,"getItemCount","()I",&[])?.i()?;
        if count<1{return Ok(None);}
        let item=env.call_method(&clip,"getItemAt",
            "(I)Landroid/content/ClipData$Item;",&[JValue::Int(0)])?.l()?;
        let text=env.call_method(&item,"coerceToText",
            "(Landroid/content/Context;)Ljava/lang/CharSequence;",
            &[JValue::Object(&activity)])?.l()?;
        if text.is_null(){return Ok(None);}
        let java=env.call_method(&text,"toString",
            "()Ljava/lang/String;",&[])?.l()?;
        if java.is_null(){return Ok(None);}
        let value:String=env.get_string(&JString::from(java))?.into();
        Ok(Some(value))
    })();
    match result{
        Ok(Some(value))=>{
            let trimmed=value.trim();
            if trimmed.is_empty(){return Err("Clipboard has no text".into());}
            // A GitHub field never needs a megabyte clipboard payload.
            if trimmed.len()>4096{return Err("Clipboard text is too long".into());}
            Ok(trimmed.into())
        }
        Ok(None)=>Err("Clipboard is empty or unavailable; focus Map Studio first".into()),
        Err(_)=>{
            let _=env.exception_clear();
            Err("Android denied clipboard access; keep Map Studio in the foreground".into())
        }
    }
}

fn put_string(env: &mut JNIEnv<'_>, values: &JObject<'_>, key: &str, value: &str)
    -> jni::errors::Result<()> {
    let k = JObject::from(env.new_string(key)?);
    let v = JObject::from(env.new_string(value)?);
    env.call_method(values, "put", "(Ljava/lang/String;Ljava/lang/String;)V",
        &[JValue::Object(&k), JValue::Object(&v)])?;
    Ok(())
}

fn put_int(env: &mut JNIEnv<'_>, values: &JObject<'_>, key: &str, value: i32)
    -> jni::errors::Result<()> {
    let k = JObject::from(env.new_string(key)?);
    let boxed = env.call_static_method("java/lang/Integer", "valueOf",
        "(I)Ljava/lang/Integer;", &[JValue::Int(value)])?.l()?;
    env.call_method(values, "put", "(Ljava/lang/String;Ljava/lang/Integer;)V",
        &[JValue::Object(&k), JValue::Object(&boxed)])?;
    Ok(())
}

fn write_download(env: &mut JNIEnv<'_>, activity: &JObject<'_>, contents: &[u8], name: &str)
    -> jni::errors::Result<()> {
    let sdk = env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?.i()?;
    if sdk < 29 {
        return Err(jni::errors::Error::NullPtr("MediaStore Downloads requires API 29"));
    }
    let resolver=env.call_method(activity, "getContentResolver",
        "()Landroid/content/ContentResolver;", &[])?.l()?;
    let values=env.new_object("android/content/ContentValues", "()V", &[])?;
    put_string(env, &values, "_display_name", name)?;
    put_string(env, &values, "mime_type",
        if name.ends_with(".json") { "application/json" } else { "application/zip" })?;
    put_string(env, &values, "relative_path", "Download/MapStudio")?;
    put_int(env, &values, "is_pending", 1)?;
    let collection=env.get_static_field("android/provider/MediaStore$Downloads",
        "EXTERNAL_CONTENT_URI", "Landroid/net/Uri;")?.l()?;
    let uri=env.call_method(&resolver, "insert",
        "(Landroid/net/Uri;Landroid/content/ContentValues;)Landroid/net/Uri;",
        &[JValue::Object(&collection),JValue::Object(&values)])?.l()?;
    if uri.is_null() {
        return Err(jni::errors::Error::NullPtr("MediaStore.insert returned null"));
    }
    let result=(|| -> jni::errors::Result<()> {
        let stream=env.call_method(&resolver, "openOutputStream",
            "(Landroid/net/Uri;)Ljava/io/OutputStream;",
            &[JValue::Object(&uri)])?.l()?;
        if stream.is_null() {
            return Err(jni::errors::Error::NullPtr("MediaStore.openOutputStream returned null"));
        }
        let written=(|| -> jni::errors::Result<()> {
            for chunk in contents.chunks(32*1024) {
                let array=env.byte_array_from_slice(chunk)?;
                let array_obj=JObject::from(array);
                env.call_method(&stream, "write", "([B)V",
                    &[JValue::Object(&array_obj)])?;
                env.delete_local_ref(array_obj)?;
            }
            env.call_method(&stream, "flush", "()V", &[])?;
            Ok(())
        })();
        let closed=env.call_method(&stream, "close", "()V", &[]);
        written?;
        closed?;
        env.call_method(&values, "clear", "()V", &[])?;
        put_int(env, &values, "is_pending", 0)?;
        let null=JObject::null();
        env.call_method(&resolver, "update",
            "(Landroid/net/Uri;Landroid/content/ContentValues;Ljava/lang/String;[Ljava/lang/String;)I",
            &[JValue::Object(&uri),JValue::Object(&values),
              JValue::Object(&null),JValue::Object(&null)])?;
        Ok(())
    })();
    if result.is_err() {
        let _=env.exception_clear();
        let null=JObject::null();
        let _=env.call_method(&resolver, "delete",
            "(Landroid/net/Uri;Ljava/lang/String;[Ljava/lang/String;)I",
            &[JValue::Object(&uri),JValue::Object(&null),JValue::Object(&null)]);
    }
    result
}

pub fn publish(source: &Path, name: &str) -> Result<String,String> {
    let bytes=std::fs::read(source).map_err(|e|format!("Read export: {e}"))?;
    let app=ANDROID_APP.get().ok_or("Android Activity unavailable")?;
    // SAFETY: the Android app owns the Java VM pointer.
    let vm=unsafe {JavaVM::from_raw(app.vm_as_ptr().cast())}
        .map_err(|e|format!("Java VM: {e}"))?;
    let mut env=vm.attach_current_thread().map_err(|e|format!("Attach JNI thread: {e}"))?;
    // SAFETY: this is an unowned global reference managed by AndroidApp.
    let activity=unsafe {JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject)};
    write_download(&mut env,&activity,&bytes,name)
        .map_err(|e|format!("Android Downloads: {e}"))?;
    Ok(format!("Downloads/MapStudio/{name}"))
}


/// Browser entries represent only MediaStore files that the current APK can read.
/// Android does not grant blanket access to other apps' Downloads.
#[derive(Clone,Debug)]
pub struct BackupEntry {
    pub id:i64,
    pub name:String,
}

/// Lists this app's own JSON backups in Downloads/MapStudio, newest first.
pub fn list_backups()->Result<Vec<BackupEntry>,String> {
    let app=ANDROID_APP.get().ok_or("Android Activity unavailable")?;
    let vm=unsafe {JavaVM::from_raw(app.vm_as_ptr().cast())}
        .map_err(|e|format!("Java VM: {e}"))?;
    let mut env=vm.attach_current_thread().map_err(|e|format!("Attach JNI: {e}"))?;
    let activity=unsafe {JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject)};
    let query=(||->jni::errors::Result<Vec<BackupEntry>>{
        let resolver=env.call_method(&activity,"getContentResolver",
            "()Landroid/content/ContentResolver;",&[])?.l()?;
        let collection=env.get_static_field("android/provider/MediaStore$Downloads",
            "EXTERNAL_CONTENT_URI","Landroid/net/Uri;")?.l()?;
        let selection=JObject::from(env.new_string("relative_path = ?")?);
        let args=env.new_object_array(1,"java/lang/String",JObject::null())?;
        let relative_path=JObject::from(env.new_string("Download/MapStudio/")?);
        env.set_object_array_element(&args,0,&relative_path)?;
        let sort=JObject::from(env.new_string("date_added DESC")?);
        let none=JObject::null();
        let cursor=env.call_method(&resolver,"query",
            "(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;)Landroid/database/Cursor;",
            &[JValue::Object(&collection),JValue::Object(&none),
              JValue::Object(&selection),JValue::Object(args.as_ref()),
              JValue::Object(&sort)])?.l()?;
        if cursor.is_null() {
            return Err(jni::errors::Error::NullPtr("MediaStore.query returned null"));
        }
        let rows=(||->jni::errors::Result<Vec<BackupEntry>>{
            let id_key=JObject::from(env.new_string("_id")?);
            let name_key=JObject::from(env.new_string("_display_name")?);
            let id_col=env.call_method(&cursor,"getColumnIndexOrThrow",
                "(Ljava/lang/String;)I",&[JValue::Object(&id_key)])?.i()?;
            let name_col=env.call_method(&cursor,"getColumnIndexOrThrow",
                "(Ljava/lang/String;)I",&[JValue::Object(&name_key)])?.i()?;
            let mut out=Vec::new();
            while env.call_method(&cursor,"moveToNext","()Z",&[])?.z()? {
                let id=env.call_method(&cursor,"getLong","(I)J",
                    &[JValue::Int(id_col)])?.j()?;
                let label=env.call_method(&cursor,"getString",
                    "(I)Ljava/lang/String;",&[JValue::Int(name_col)])?.l()?;
                if label.is_null() {continue;}
                let name:String=env.get_string(&JString::from(label))?.into();
                if (name.starts_with("MapStudio-project-") && name.ends_with(".json"))
                    || (name.starts_with("MapStudio-maps-") && name.ends_with(".zip"))
                    || (name.starts_with("MapStudio-editable-") && name.ends_with(".zip"))
                    || name.ends_with(".zip") {
                    out.push(BackupEntry{id,name});
                }
                if out.len()>=80 {break;}
            }
            Ok(out)
        })();
        let _=env.call_method(&cursor,"close","()V",&[]);
        rows
    })();
    query.map_err(|e|format!("Browse Downloads: {e}"))
}

/// Reads a selected own-app MediaStore backup, bounded to avoid memory exhaustion.
pub fn read_backup(id:i64)->Result<Vec<u8>,String> {
    if id<0 {return Err("Invalid backup ID".into());}
    let app=ANDROID_APP.get().ok_or("Android Activity unavailable")?;
    let vm=unsafe {JavaVM::from_raw(app.vm_as_ptr().cast())}
        .map_err(|e|format!("Java VM: {e}"))?;
    let mut env=vm.attach_current_thread().map_err(|e|format!("Attach JNI: {e}"))?;
    let activity=unsafe {JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject)};
    let result=(||->jni::errors::Result<Vec<u8>>{
        let resolver=env.call_method(&activity,"getContentResolver",
            "()Landroid/content/ContentResolver;",&[])?.l()?;
        let collection=env.get_static_field("android/provider/MediaStore$Downloads",
            "EXTERNAL_CONTENT_URI","Landroid/net/Uri;")?.l()?;
        let uri=env.call_static_method("android/content/ContentUris","withAppendedId",
            "(Landroid/net/Uri;J)Landroid/net/Uri;",
            &[JValue::Object(&collection),JValue::Long(id)])?.l()?;
        let stream=env.call_method(&resolver,"openInputStream",
            "(Landroid/net/Uri;)Ljava/io/InputStream;",
            &[JValue::Object(&uri)])?.l()?;
        if stream.is_null(){
            return Err(jni::errors::Error::NullPtr("MediaStore InputStream is null"));
        }
        let bytes=(||->jni::errors::Result<Vec<u8>>{
            let buffer=env.new_byte_array(16384)?;
            let mut data=Vec::new();
            let mut chunk=vec![0i8;16384];
            loop {
                let count=env.call_method(&stream,"read","([B)I",
                    &[JValue::Object(buffer.as_ref())])?.i()?;
                if count<0 {break;}
                if count==0 {continue;}
                if data.len().saturating_add(count as usize)>crate::artwork::MAX_PACKAGE_BYTES {
                    return Err(jni::errors::Error::NullPtr("Project backup is too large"));
                }
                env.get_byte_array_region(&buffer,0,&mut chunk[..count as usize])?;
                data.extend(chunk[..count as usize].iter().map(|b|*b as u8));
            }
            Ok(data)
        })();
        let _=env.call_method(&stream,"close","()V",&[]);
        bytes
    })();
    result.map_err(|e|format!("Read Downloads backup: {e}"))
}
