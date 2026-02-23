## Reply Draft (Copy/Paste)

Thanks for the very concrete review - that matches the real friction points almost perfectly.

A few things are intentionally built as Alpha/MVP tradeoffs, but I agree with your priorities:

- The dual backend is deliberately framed as "tauri first, chromium when needed", not as a defeat. The README now includes a simple decision rule and concrete examples.
- Build time is currently the biggest hurdle for time-to-first-success. This is exactly why the Chromium backend exists: many users want "works now" instead of a 2-5 minute local build plus prerequisites.
- Config persistence is the next logical step: update without having to re-specify URL/config should be the default once metadata is stored under `~/.local/share/deskify/`.
- Distribution: CI and a tag-based GitHub Release workflow already produce a Linux binary asset. Next step is to make "install from releases" more prominent and enable community packaging (e.g. AUR `-bin`).

Thanks also for calling out the WebKitGTK reality. That's why the backend story is explicit: system WebView where it works well, Chromium app mode when a site needs a newer engine.

If you're up for it, I'd love to keep you as a sparring partner specifically on time-to-first-success (docs/install/release), since that's often more decisive than feature count in OSS adoption.

