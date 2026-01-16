Check release status or trigger a manual release.

Usage: /release [status|trigger|check]

This command helps you:
- Check if there are unreleased changes
- View pending version bump type
- Trigger a manual release if needed

---

I'll help you with the release process. Let me check the current state:

1. **Current version** from VERSION file
2. **Commits since last tag**
3. **Predicted version bump** based on commit types
4. **CI status** for main branch

What would you like to do?

Options:
- **status** - Show current version and unreleased changes
- **check** - Verify VERSION and package.json are in sync
- **trigger** - Manually create a release tag (if auto-release failed)

Default action is "status" - I'll show you what's pending for release.
