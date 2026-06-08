# Codex Beacon 发布流水线方案

Date: 2026-06-08
Status: Draft

## 目标

Codex Beacon 使用两条发布通道：

- `master` 预览通道：代码合入 `master` 后，GitHub Actions 自动构建 macOS 安装包，并覆盖固定的 `preview` 预览版 Release。
- `vX.Y.Z` 正式通道：推送形如 `vX.Y.Z` 的 tag 后，GitHub Actions 自动构建 macOS 安装包，并把 `.dmg` 上传到对应的正式 GitHub Release。

## 设计原则

- 正式 Release 由版本 tag 驱动，不手动上传本地安装包。
- 预览 Release 由 `master` 分支驱动，始终覆盖同一个 `preview` tag 和 Release。
- Release artifact 必须能追溯到唯一 commit。
- macOS 用户优先下载 `.dmg`。
- 正式 release 必须支持 code signing 和 notarization。
- 没有 Apple Developer 凭据时，只允许生成 preview/prerelease 或 CI artifact，用于内部验证。

## Release 通道约定

预览版本：

```text
branch: master
release tag: preview
release title: Codex Beacon Preview
release type: prerelease
latest: false
asset naming: Codex.Beacon-preview-${arch}.dmg
```

每次 `master` 更新都会：

- 构建新的 macOS DMG。
- 强制移动远端 `preview` tag 到当前 `master` commit。
- 创建或更新 `preview` GitHub Release。
- 用 `gh release upload preview ... --clobber` 覆盖固定命名的 DMG asset。
- 重写 Release notes，记录 commit SHA、构建时间和 workflow run 链接。

`preview` 是可变发布，不作为用户长期回滚入口。需要长期保存的版本必须走 `vX.Y.Z` 正式通道。

正式版本：

稳定版本：

```bash
git tag -a v0.1.0 -m "v0.1.0"
git push origin v0.1.0
```

预发布版本：

```bash
git tag -a v0.1.0-alpha.1 -m "v0.1.0-alpha.1"
git push origin v0.1.0-alpha.1
```

后续可以加一个 `scripts/release.sh v0.1.0`，统一检查工作树、版本号、tag 是否存在、是否已 push。

## GitHub Actions 触发

预览 workflow 使用 `master` branch push 触发。合入 `master` 后会产生 push 事件，所以不需要依赖 PR merge event：

```yaml
on:
  push:
    branches:
      - "master"
```

正式 workflow 使用 tag push 触发：

```yaml
on:
  push:
    tags:
      - "v*"
```

GitHub 文档支持在 `push` 事件上用 `branches` 过滤分支、用 `tags` 过滤 tag。Release workflow 需要 `contents: write` 权限，以便创建 GitHub Release、移动 preview tag 并上传 assets。

## Tauri DMG 构建

Tauri 官方 DMG 构建命令：

```bash
pnpm tauri build --bundles dmg
```

等项目 scaffold 后，workflow 优先使用 `tauri-apps/tauri-action@v1`。该 action 可以构建 Tauri app，创建 GitHub Release，并上传 bundle artifacts。

推荐先支持两种实现路径：

1. **Action 托管发布**
   使用 `tauri-apps/tauri-action@v1`，由 action 创建/更新 release 并上传 artifacts。实现最短，适合 MVP。

2. **手动上传发布**
   手动执行 `pnpm tauri build --bundles dmg`，再用 `gh release create/upload` 上传 DMG。适合作为本地排障或 action 无法满足签名流程时的备用路径。

## macOS 架构策略

优先目标：一个 universal DMG。

```bash
pnpm tauri build --target universal-apple-darwin --bundles dmg
```

Tauri CLI 支持 `universal-apple-darwin`，但要求同时安装 `aarch64-apple-darwin` 和 `x86_64-apple-darwin` Rust targets。

备选目标：两个架构各出一个 DMG。

```text
Codex.Beacon_0.1.0_aarch64.dmg
Codex.Beacon_0.1.0_x64.dmg
```

MVP 决策建议：

- 第一版 CI 先构建 `aarch64-apple-darwin` 和 `x86_64-apple-darwin` 两个 DMG，稳定后再切 universal。
- 若 universal 在 GitHub macOS runner 上稳定，则 release 只保留一个 `universal.dmg`，降低用户选择成本。

## 签名和公证

正式 macOS release 需要 Apple Developer ID 签名和 notarization，否则用户下载后可能遇到 Gatekeeper 阻止或安全提示。

需要的 GitHub Secrets：

```text
APPLE_CERTIFICATE
APPLE_CERTIFICATE_PASSWORD
APPLE_SIGNING_IDENTITY
APPLE_API_ISSUER
APPLE_API_KEY
APPLE_API_KEY_PATH or APPLE_API_KEY_CONTENT
```

实现时需要把私钥内容写入 CI 临时文件，再把路径传给 Tauri notarization 环境变量。

发布等级：

- `unsigned`: 只用于本地或内部验证，不标记 latest。
- `signed`: 已签名但未公证，只用于临时验证。
- `notarized`: 正式发布，可标记 latest。

## Workflow 草案

项目 scaffold 后落地为两个 workflow：

- `.github/workflows/preview.yml`: `master` 合入后覆盖 `preview` 预览版。
- `.github/workflows/release.yml`: `vX.Y.Z` tag 后发布正式版。

### Preview workflow 草案

```yaml
name: preview

on:
  push:
    branches:
      - "master"

permissions:
  contents: write

jobs:
  build-macos-dmg:
    runs-on: macos-latest
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: aarch64-apple-darwin
            arch: aarch64
          - target: x86_64-apple-darwin
            arch: x64

    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - uses: actions/setup-node@v4
        with:
          node-version: lts/*

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin,x86_64-apple-darwin

      - name: Install frontend dependencies
        run: pnpm install --frozen-lockfile

      - name: Build DMG
        run: pnpm tauri build --target ${{ matrix.target }} --bundles dmg

      - name: Normalize asset name
        shell: bash
        run: |
          set -euo pipefail
          mkdir -p release-assets
          dmg="$(find ./apps/desktop-tauri/src-tauri/target -path '*/bundle/dmg/*.dmg' -print -quit)"
          cp "$dmg" "release-assets/Codex.Beacon-preview-${{ matrix.arch }}.dmg"

      - uses: actions/upload-artifact@v4
        with:
          name: preview-dmg-${{ matrix.arch }}
          path: release-assets/*.dmg

  publish-preview:
    runs-on: ubuntu-latest
    needs: build-macos-dmg
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - uses: actions/download-artifact@v4
        with:
          pattern: preview-dmg-*
          path: release-assets
          merge-multiple: true

      - name: Move preview tag
        run: |
          git tag -f preview "$GITHUB_SHA"
          git push origin refs/tags/preview --force

      - name: Create or update preview release
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          gh release view preview >/dev/null 2>&1 || \
            gh release create preview \
              --title "Codex Beacon Preview" \
              --prerelease \
              --latest=false \
              --target "$GITHUB_SHA"

          gh release edit preview \
            --title "Codex Beacon Preview" \
            --prerelease \
            --target "$GITHUB_SHA" \
            --notes "Preview build from ${GITHUB_SHA}. Workflow: ${GITHUB_SERVER_URL}/${GITHUB_REPOSITORY}/actions/runs/${GITHUB_RUN_ID}"

          gh release upload preview release-assets/*.dmg --clobber
```

Notes:

- `preview` Release 是可覆盖的；旧 DMG asset 会被同名新 asset 替换。
- preview 使用固定文件名，例如 `Codex.Beacon-preview-aarch64.dmg` 和 `Codex.Beacon-preview-x64.dmg`，避免历史 preview asset 越积越多。
- preview 拆成 `build-macos-dmg` 和 `publish-preview`，发布动作只执行一次，避免 matrix job 并发移动同一个 tag 或同时编辑同一个 Release。
- preview 默认标记为 prerelease。GitHub latest release 只选择 non-prerelease、non-draft release；创建 preview 时仍显式使用 `--latest=false`，避免污染正式 latest。

### Release workflow 草案

```yaml
name: release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

jobs:
  macos-dmg:
    runs-on: macos-latest
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: aarch64-apple-darwin
          - target: x86_64-apple-darwin

    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: lts/*

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin,x86_64-apple-darwin

      - name: Install frontend dependencies
        run: pnpm install --frozen-lockfile

      - uses: tauri-apps/tauri-action@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
          APPLE_API_ISSUER: ${{ secrets.APPLE_API_ISSUER }}
          APPLE_API_KEY: ${{ secrets.APPLE_API_KEY }}
          APPLE_API_KEY_PATH: ${{ secrets.APPLE_API_KEY_PATH }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: "Codex Beacon ${{ github.ref_name }}"
          releaseBody: "See attached DMG assets to install Codex Beacon."
          releaseDraft: false
          prerelease: ${{ contains(github.ref_name, '-') }}
          args: "--target ${{ matrix.target }} --bundles dmg"
```

Notes:

- 这个 workflow 是草案，等 `apps/desktop-tauri` scaffold 后需要补 `projectPath`、包管理器和 frontend build 命令。
- 如果采用 universal DMG，matrix 可替换为单 target `universal-apple-darwin`。
- `APPLE_API_KEY_PATH` 在 GitHub Secrets 中不能直接保存文件路径；实际实现时更推荐保存 key 内容，再在 workflow 中写入临时 `.p8` 文件。

## 本地备用命令

创建 release 但要求 tag 已存在：

```bash
gh release create v0.1.0 ./apps/desktop-tauri/src-tauri/target/**/bundle/dmg/*.dmg \
  --verify-tag \
  --generate-notes \
  --title "Codex Beacon v0.1.0"
```

`gh release create` 可以直接上传 asset；加 `--verify-tag` 可以避免意外从默认分支自动创建 tag。

## 参考依据

- [Tauri DMG distribution](https://v2.tauri.app/distribute/dmg/): DMG 是 macOS App Store 外分发的常见安装格式，Tauri CLI 可用 `tauri build --bundles dmg` 生成。
- [Tauri GitHub Action](https://github.com/tauri-apps/tauri-action): `tauri-apps/tauri-action@v1` 可构建 Tauri app，并创建 GitHub Release、上传 bundle artifacts。
- [GitHub Actions push tag trigger](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#running-your-workflow-only-when-a-push-of-specific-tags-occurs): `push.tags` 可让 workflow 只在 tag push 时运行。
- [GitHub Actions push branch trigger](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#running-your-workflow-only-when-a-push-to-specific-branches-occurs): `push.branches` 可让 workflow 只在指定分支 push 时运行。
- [GitHub CLI `gh release create`](https://cli.github.com/manual/gh_release_create): `gh release create` 可以创建 release 并上传 asset，`--verify-tag` 可要求远端 tag 已存在。
- [GitHub CLI `gh release upload`](https://cli.github.com/manual/gh_release_upload): `--clobber` 会删除并重新上传同名 release asset，适合覆盖 preview DMG。
- [GitHub Releases REST API](https://docs.github.com/en/rest/releases/releases): GitHub 的 latest 选择会排除 draft 和 prerelease，因此 preview 应保持 prerelease。
