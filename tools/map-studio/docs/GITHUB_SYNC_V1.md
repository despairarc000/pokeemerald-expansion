# GitHub project synchronization (V1)

## Actual status

The Android app includes an opt-in GitHub Contents API client for the
**selected project only**. GitHub repositories (public or private) may
contain the shared paths below:

\`\`\`text
Editor/projects/<project-uuid>/manifest.json
Editor/projects/<project-uuid>/project.json
\`\`\`

No webhooks, Supabase, custom public server or relay is required for polling.
The application must be open to check for updates; background syncing or
multi-user collaboration is not implemented.

## First pairing

1. In Map Studio, create/open a project. The project has a permanent UUID.
2. Open **Project → GitHub sync**, enter the target \`owner/repository\` and
   branch. A separate projects repository is recommended over the game fork.
3. For a private repo or to publish, paste a **fine-grained, repository-scoped
   GitHub token with Contents write permission** into the password field.
   Use a restricted token. Never insert a token into project.json or GitHub
   commits. The app keeps the token in memory only and clears it on project
   switching/restart; it does not persist credentials.
4. Tap **Publish**. This publishes the matching manifest and current
   \`project.json\` to that UUID directory. If the directory already contains
   a different project, the app rejects overwriting it.
5. AI agents can fetch and edit the matching \`Editor/projects/<UUID>/project.json\`
   through an explicitly authorized GitHub connector. They must verify the
   manifest target, use the shared Rust project rules, avoid crossing UUIDs,
   and publish one validated revision with GitHub SHA preconditions.
6. Select **Auto-check while editor is open** (45 seconds with a token,
   5 minutes without), or tap **Pull now**. When a newer compatible map
   revision is available, the app validates it, creates a local recovery
   backup, and loads it into the canvas.

## Conflict and safety behavior

- Stable project UUID and target manifest are validated on both ends.
- The phone stores its last remote Git blob SHA and a SHA-256 hash of the
  canonical map model. JSON whitespace does not cause false conflicts.
- Remote writes use the Contents API's previous blob SHA precondition.
  If the phone and remote both changed, syncing **stops** rather than
  discarding one set of changes. No automatic merge is attempted.
- User changes made during a background pull are detected before applying.
  Switching projects is blocked until an in-flight HTTP request completes.
- Only metadata (repo, branch, auto-check) and the per-project revision cursor
  are stored locally; the access token is never serialized.
- Public GitHub repositories expose your project data publicly. Private
  repository pulls require an in-session credential.
- The app does not automatically commit generated \`Compiled/\` files.
  Use \`workspace-publish\` for project-scoped source generation.
- GitHub's Contents API is not an efficient real-time multi-person CRDT.
  Collaboration would need a different conflict-resolution model.

## Limits and acceptance criteria

Building a signed APK and passing mock-free model tests does **not** prove
network behavior on the owner's physical Android device. Before calling the
feature phone-verified, test authenticating to a repository, publishing a
new UUID project, editing the actual JSON through an AI, pulling the new
revision, checking offline behavior, and provoking conflicts on a device.

The app can now clone an existing GitHub project **by explicit UUID**.
From the GitHub Sync dialog, enter the repository, branch and project ID
found in `Editor/projects/<UUID>/`, then select **Clone existing project**.
The manifest and world are validated before a create-only local import;
existing local UUID directories are never overwritten. This lets the
v0.18 Emerald Rebuild Preview reconnect to a project from another Android
package without uninstalling it. Automatic remote project discovery is
still not implemented.
