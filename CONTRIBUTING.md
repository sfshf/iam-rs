# CONTRIBUTING

## Git Commit Message Guidelines

We follow the [Conventional Commits](https://conventionalcommits.org) specification for all repository contributions.

### Format

Commit messages must follow this format:
`<type>(<scope>): <subject>`

### Allowed Types

- **feat**: A new feature for the user (translates to a MINOR version change)
- **fix**: A bug fix for the user (translates to a PATCH version change)
- **docs**: Documentation only changes (e.g., updates to README.md)
- **style**: Changes that do not affect the meaning of the code (white-space, formatting, missing semi-colons, etc.)
- **refactor**: A code change that neither fixes a bug nor adds a feature
- **test**: Adding missing tests or correcting existing tests
- **chore**: Changes to the build process or auxiliary tools and libraries (e.g., dependency updates)

### Examples

- `feat(auth): add multi-factor authentication via SMS`
- `fix(dashboard): resolve memory leak in real-time chart component`
- `docs(readme): update installation instructions for local development`
