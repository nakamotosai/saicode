# Saicode Historical Residue Removal

## Goal

彻底移除历史第二入口、已废弃会话守护实现与对应桥接残留，使 `saicode` 只保留一个真实用户入口 `./bin/saicode`，并把当前仓库从“巨大 cutover 脏工作树”收敛到可提交、可验证、`git status` 干净的状态。

## Scope

### In scope

- 删除已废弃会话守护 crate
- 从 `rust/Cargo.toml`、`Cargo.lock`、脚本、README、Spec/Plan、安装链路中移除旧守护路径残留
- 清理旧第二入口、桥接 prompt、专属命令/文案/帮助残留
- 更新相关测试与 closeout 脚本
- 清理任务产生的临时目录与无效残留
- 在验证通过后收敛 git 工作树到干净状态

### Out of scope

- 浏览器/视觉前端修复
- 新功能扩张
- 历史文档的逐字逐句全面重写；只处理会误导当前实现或阻碍仓库收口的残留

## Constraints

- 不把“文件删掉了”误判成“链路收口了”
- 不保留第二入口或第二套系统 prompt 真相
- 不让帮助页、脚本、README 继续提不存在的入口或 crate
- 没有验证，不算移除完成
- 收口仓库脏状态时，不做破坏性回滚；以当前 Rust cutover 为真实主线整理并提交

## Acceptance

1. 仓库内不再存在已废弃会话守护 crate 或对它的活跃构建引用
2. 仓库内不再存在旧第二入口或其系统 prompt 残留
3. `./bin/saicode` 的 help/status/print/interactive slash 基本面仍可正常工作
4. 相关 cargo test/build 与 closeout smoke 通过
5. README 与当前入口、默认模型、测试命令一致
6. 任务结束时 `git status --short` 干净

## Risks

- 旧守护路径删除后可能暴露出桥接或测试对旧协议的隐式依赖
- 当前仓库脏工作树很大，若不分辨临时垃圾和真实 cutover 产物，容易误删
- 历史 spec 若保留过多旧口径，会继续误导后续维护
