Start a new feature branch for development.

Usage: /new-feature <type> <description>

Arguments:
- type: feat, fix, perf, refactor, docs, chore, test
- description: Short kebab-case description (e.g., "add-dark-mode")

This command will:
1. Ensure you're on latest main
2. Create a properly named branch
3. Set up tracking with origin

Example: /new-feature feat add-keyboard-shortcuts

---

I'll help you start a new feature branch. First, let me check the current git status and create the branch.

What type of change is this and what's a short description?

Types:
- **feat** - New feature (minor version bump)
- **fix** - Bug fix (patch version bump)
- **perf** - Performance improvement (patch)
- **refactor** - Code restructuring (patch)
- **docs** - Documentation only (no version bump)
- **chore** - Maintenance (no version bump)
- **test** - Test changes (no version bump)

After you provide the type and description, I'll:
1. Pull latest main
2. Create branch `{type}/{description}`
3. Push and set upstream

Example response: "feat add-settings-export" or "fix memory-leak-detection"
