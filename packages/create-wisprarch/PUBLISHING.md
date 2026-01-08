# Publishing create-wisprarch to npm

## Prerequisites

1. **npm account**: Create one at https://www.npmjs.com/signup
2. **npm login**: Run `npm login` and enter your credentials
3. **2FA setup**: Recommended for security

## Publishing Steps

### 1. Login to npm

```bash
npm login
```

Enter your npm username, password, and email.

### 2. Verify you're logged in

```bash
npm whoami
```

Should display your npm username.

### 3. Build the package

```bash
cd packages/create-wisprarch
bun run build
```

### 4. Test locally (optional)

```bash
# Test the built package
node dist/index.js

# Or test with npm link
npm link
create-wisprarch
npm unlink
```

### 5. Publish to npm

```bash
npm publish
```

For first-time publish of a scoped package, use:
```bash
npm publish --access public
```

### 6. Verify publication

```bash
npm view create-wisprarch
```

### 7. Test the published package

```bash
npx create-wisprarch@latest
```

## Updating the Package

When you make changes:

1. Update version in `package.json`:
   ```bash
   npm version patch  # for bug fixes (0.1.0 -> 0.1.1)
   npm version minor  # for new features (0.1.0 -> 0.2.0)
   npm version major  # for breaking changes (0.1.0 -> 1.0.0)
   ```

2. Rebuild:
   ```bash
   bun run build
   ```

3. Commit and tag:
   ```bash
   git add .
   git commit -m "chore: bump version to $(node -p "require('./package.json').version")"
   git push
   ```

4. Publish:
   ```bash
   npm publish
   ```

## Automated Publishing with GitHub Actions (Optional)

Create `.github/workflows/publish-npm.yml`:

```yaml
name: Publish to npm

on:
  release:
    types: [created]

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1
      - run: cd packages/create-wisprarch && bun install
      - run: cd packages/create-wisprarch && bun run build
      - uses: JS-DevTools/npm-publish@v3
        with:
          token: ${{ secrets.NPM_TOKEN }}
          package: packages/create-wisprarch/package.json
```

Then add your npm token to GitHub secrets:
1. Generate token at https://www.npmjs.com/settings/YOUR_USERNAME/tokens
2. Add to GitHub: Settings → Secrets → Actions → New repository secret
3. Name: `NPM_TOKEN`, Value: your token

## Troubleshooting

### "need auth" error
Run `npm login` first

### "package already exists" error
The package name is taken. Choose a different name in `package.json`.

### "403 Forbidden" error
- Check if you have permission to publish (must be logged in)
- For scoped packages (@username/package), use `npm publish --access public`

### Version already published
Bump the version in `package.json` before publishing

## Package Maintenance

- Keep dependencies updated: `bun update`
- Test thoroughly before each release
- Follow semantic versioning
- Update README.md with any new features
- Respond to issues and PRs promptly

## Support

If you encounter issues, check:
- npm documentation: https://docs.npmjs.com/
- npm support: https://npmjs.com/support
