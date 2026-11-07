//! Publish native map exports and project backups to user-accessible Downloads.
//! Android 10+ MediaStore requires no broad storage permissions.
use jni::{objects::{JObject, JValue}, JNIEnv, JavaVM};
use std::{path::Path, sync::OnceLock};

static ANDROID_APP: OnceLock<winit::platform::android::activity::AndroidApp> = OnceLock::new();

pub fn set_app(app: winit::platform::android::activity::AndroidApp) {
    let _ = ANDROID_APP.set(app);
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
