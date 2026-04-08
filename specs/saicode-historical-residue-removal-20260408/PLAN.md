# Plan

## Phase 1. Inventory

- 定位历史第二入口、旧守护 crate、桥接 prompt、脚本、README、spec 中的所有残留
- 盘点当前 git 脏工作树，区分：
  - 当前真实 Rust 主线产物
  - 任务临时目录/垃圾
  - 仍引用旧入口的残留

## Phase 2. Removal

- 删除旧守护 crate
- 从 workspace、脚本、README、测试命令、spec 中移除对应引用
- 删除旧第二入口和桥接 prompt 残留
- 收紧命令帮助与提示，只保留 `saicode` 当前真实支持面

## Phase 3. Verification

- 运行相关 cargo test/build
- 运行 `./bin/saicode` 的 help/status/print/interactive slash smoke
- 运行 closeout preflight

## Phase 4. Dirty Repo Closure

- 删除任务临时目录与无效垃圾
- 同步 README / SPEC / PLAN
- 将当前 Rust cutover 主线整理为一次或少量提交
- 确保 `git status --short` 干净
