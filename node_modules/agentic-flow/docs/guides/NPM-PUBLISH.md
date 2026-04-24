# Publishing agent-flow to npm

## Prerequisites

1. npm account with publish permissions
2. Verify package.json correctness
3. Complete build and testing
4. Update version number

## Pre-Publish Checklist

```bash
# 1. Clean and rebuild
npm run build

# 2. Test locally
node dist/cli.js --help
node dist/cli.js --list

# 3. Test in Docker
docker build -t agent-flow:test .
docker run --rm -e ANTHROPIC_API_KEY=test agent-flow:test --help

# 4. Check package contents
npm pack --dry-run

# 5. Verify .npmignore excludes dev files
cat .npmignore
```

## Version Update

```bash
# Update version (choose one)
npm version patch  # 1.0.0 -> 1.0.1
npm version minor  # 1.0.0 -> 1.1.0
npm version major  # 1.0.0 -> 2.0.0

# Or manually edit package.json version field
```

## Publishing Steps

### 1. Login to npm

```bash
npm login
# Enter credentials when prompted
```

### 2. Dry Run (Test Publish)

```bash
npm publish --dry-run
```

Review the output to ensure correct files are included.

### 3. Publish to npm

```bash
# Public package (free)
npm publish --access public

# Or for scoped package
npm publish
```

### 4. Verify Publication

```bash
# Check on npmjs.com
open https://www.npmjs.com/package/agent-flow

# Test installation
npx agent-flow@latest --help
```

## Post-Publish

### 1. Tag Release in Git

```bash
git tag v1.0.0
git push origin v1.0.0
```

### 2. Create GitHub Release

1. Go to https://github.com/ruvnet/agent-flow/releases
2. Click "Draft a new release"
3. Select tag v1.0.0
4. Title: "Agent Flow v1.0.0"
5. Describe changes from CHANGELOG.md
6. Attach binaries if applicable
7. Publish release

### 3. Update Documentation

- Update README with npm install instructions
- Update version badges
- Announce on social media

## Unpublishing (Emergency Only)

```bash
# Unpublish within 72 hours
npm unpublish agent-flow@1.0.0

# Deprecate instead (preferred)
npm deprecate agent-flow@1.0.0 "Critical bug - use 1.0.1 instead"
```

## Troubleshooting

### Error: Package name already exists

```bash
# Check if name is taken
npm search agent-flow

# Use scoped package instead
# Update package.json: "name": "@your-org/agent-flow"
```

### Error: 402 Payment Required

- Scoped packages require paid npm account or public access flag
- Use: `npm publish --access public`

### Error: 403 Forbidden

- Not logged in: `npm login`
- No permissions: Contact package owner
- Email not verified: Check npm email

## Beta/Alpha Releases

```bash
# Publish pre-release versions
npm version 1.1.0-alpha.1
npm publish --tag alpha

# Users install with:
npm install agent-flow@alpha
```

## Package Maintenance

```bash
# View published versions
npm view agent-flow versions

# Add collaborators
npm owner add <username> agent-flow

# Check package health
npm doctor

# View download stats
npm info agent-flow
```

## Files Included in Package

Based on `.npmignore` and `package.json` "files" field:

**Included:**
- `dist/` - Compiled JavaScript
- `docs/` - Documentation
- `.claude/` - Agent definitions
- `README.md` - Package description
- `LICENSE` - MIT license
- `package.json` - Package manifest

**Excluded:**
- `src/` - TypeScript source (after build)
- `node_modules/` - Dependencies
- `.env*` - Environment files
- `test-*.js` - Test files
- `validation/` - Validation scripts
- Docker files
- IDE config files

## Continuous Deployment

For automated publishing via GitHub Actions:

```yaml
# .github/workflows/publish.yml
name: Publish to npm

on:
  release:
    types: [created]

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'
          registry-url: 'https://registry.npmjs.org'
      - run: npm ci
      - run: npm test
      - run: npm run build
      - run: npm publish --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

## Support

- npm documentation: https://docs.npmjs.com/
- npm support: https://www.npmjs.com/support
- Package issues: https://github.com/ruvnet/agent-flow/issues
