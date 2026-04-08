# saicode 完整体检报告

日期：
- 2026-04-06

口径：
- 以“当前用户现在能不能直接稳定使用”为主口径
- 同时补查结构性风险、门禁覆盖、文档脚本一致性与残留问题

## 执行范围

本轮已按体检计划实际执行：

- Gate Health
- Entrypoint Health
- Task Matrix
- Runtime Semantics
- Structural Health

## 已执行的关键验证

### Gate

- `bun test`
- `bun run typecheck`
- `bun run typecheck:full`
- `bun run verify`
- `bun run rust:test:frontline`
- `cargo test --manifest-path native/saicode-launcher/Cargo.toml`

### Entrypoint

- `./scripts/closeout_preflight.sh`
- `SAICODE_CLOSEOUT_LIVE=1 ./scripts/closeout_preflight.sh`
- `~/.local/bin/saicode --version`
- `~/.local/bin/saicode -p 'Reply with exactly: ok'`
- `cd ~ && saicode`

### Task Matrix

已实测 10 类任务，全部通过：

1. 纯文本 one-shot
2. `Read`
3. `Grep`
4. `Glob + Read`
5. `Write`
6. `Read + Edit`
7. `Bash` readonly
8. `Bash` bypass
9. `WebSearch`
10. `WebFetch`

## 关键结果

### Gate Results

- `bun test`
  - 通过
  - `elapsed=2.07s`
  - `rss_kb=183076`
- `bun run typecheck`
  - 通过
  - `elapsed=1.47s`
  - `rss_kb=319236`
- `bun run typecheck:full`
  - 通过
  - `elapsed=8.21s`
  - `rss_kb=981820`
- `bun run verify`
  - 通过
  - `elapsed=9.41s`
  - `rss_kb=983312`

### Entrypoint Results

- installed command 可直接从 `~` 使用
- `saicode --help` 正常
- `saicode --version` 正常
- `saicode -p 'Reply with exactly: ok'` 正常，约 `1.76s`
- 交互界面从 `~` 启动约 `2.2s` 内可见
- `closeout_preflight` 与 `live probe` 均通过

### Task Matrix Results

- `plain_text`
  - 通过
  - `elapsed=2.508s`
  - route: `recovery`
  - target: `saicode-rust-one-shot`
- `Read`
  - 通过
  - `elapsed=3.561s`
  - route: `native-local-tools`
  - target: `saicode-rust-local-tools`
- `Grep`
  - 通过
  - `elapsed=3.931s`
  - route: `native-local-tools`
  - target: `saicode-rust-local-tools`
- `Glob + Read`
  - 通过
  - `elapsed=3.407s`
- `Write`
  - 通过
  - `elapsed=2.951s`
- `Read + Edit`
  - 通过
  - `elapsed=6.761s`
- `Bash` readonly
  - 通过
  - `elapsed=1.961s`
- `Bash` bypass
  - 通过
  - `elapsed=3.128s`
- `WebSearch`
  - 通过
  - `elapsed=4.825s`
- `WebFetch`
  - 通过
  - `elapsed=5.284s`

判断：
- 当前没有发现“会卡住不动”的任务
- 本地文件类任务大多在 `2s` 到 `4s`
- 编辑类任务约 `6s+`
- Web 类任务在 `5s` 左右，属于正常网络耗时，不是异常卡死

## 结构扫描结果

### 代码规模与覆盖现状

- `src` 下 TS/TSX 文件约 `1950`
- `rust` 下 Rust 源文件约 `208`
- 体检统计口径下合计代码文件约 `2174`
- `tests/` 顶层测试文件约 `9`
- `@ts-nocheck` 文件数为 `227`

### donor / 旧品牌残留

原先集中在 active crate 树下的 donor 残留已迁入显式 archive：

- [README.md](/home/ubuntu/saicode/rust/archive/kcode-donor/README.md)
- [adapters](/home/ubuntu/saicode/rust/archive/kcode-donor/adapters)
- [bridge](/home/ubuntu/saicode/rust/archive/kcode-donor/bridge)
- [kcode-cli](/home/ubuntu/saicode/rust/archive/kcode-donor/kcode-cli)

判断：
- 当前 active `rust/crates` 目录只保留 saicode 现役 crate
- donor 代码仍保存在仓库中，但不再伪装成活跃实现的一部分

### 残留文件

未再发现 `.bak / .old / .orig / *~` 这类明显备份残留。

## Findings

### 已在体检中顺手修掉

- `P1` 默认门禁未覆盖当前主用 Rust frontline 路径
  - 现象：
    - `verify/check` 原本只跑 `bun test + typecheck:full + native launcher tests + help smoke`
    - 没有覆盖：
      - `saicode-frontline`
      - `saicode-rust-one-shot`
      - `saicode-rust-local-tools`
  - 风险：
    - 当前真实运行已经命中这些 Rust 二进制，但默认门禁却不会发现它们的回归
  - 处理：
    - 已新增 `rust:test:frontline`
    - 已把 `check/verify` 接入这条门禁
  - 证据：
    - [package.json](/home/ubuntu/saicode/package.json)

- `P2` 默认 `typecheck` 口径误导
  - 处理：
    - `typecheck` 已切到 full
    - 相关脚本语义已加测试锁定
  - 证据：
    - [package.json](/home/ubuntu/saicode/package.json)
    - [package-scripts.test.ts](/home/ubuntu/saicode/tests/package-scripts.test.ts)

- `P2` 自动化覆盖未约束 frontline 与 `@ts-nocheck` 风险增长
  - 处理：
    - `verify/check` 已纳入 Rust frontline
    - 新增 `ts-nocheck:check`
    - 预算脚本已接入默认门禁
  - 证据：
    - [package.json](/home/ubuntu/saicode/package.json)
    - [check_ts_nocheck_budget.sh](/home/ubuntu/saicode/scripts/check_ts_nocheck_budget.sh)

- `P2` donor 重残留仍作为 active workspace 成员混入主项目
  - 处理：
    - `adapters`
    - `bridge`
    - `kcode-cli`
    已从 active Rust workspace members 中移除
  - 证据：
    - [Cargo.toml](/home/ubuntu/saicode/rust/Cargo.toml)

- `P3` 备份残留文件
  - 处理：
    - `.bak` 已删除

- `P3` donor 归档代码仍挂在 active crate 路径下
  - 处理：
    - 已从 `rust/crates` 主树迁出
    - 统一归档到 `rust/archive/kcode-donor`
    - 并补 archive 说明，避免后续继续被误认成现役 crate
  - 证据：
    - [README.md](/home/ubuntu/saicode/rust/archive/kcode-donor/README.md)
    - [Cargo.toml](/home/ubuntu/saicode/rust/Cargo.toml)

- `P3` `@ts-nocheck` 门禁口径与报告口径不一致
  - 现象：
    - 报告按文件数看
    - 旧脚本实际按命中行数算
  - 处理：
    - 已统一改成按文件数统计
    - 基线收准到当前 `227`
  - 证据：
    - [check_ts_nocheck_budget.sh](/home/ubuntu/saicode/scripts/check_ts_nocheck_budget.sh)

### 当前仍存在的长期技术债

- `@ts-nocheck` 文件数仍为 `227`
  - 现象：
    - 当前数量依然不低
  - 判断：
    - 现在已经有文件级 budget gate 锁定，属于“受控长期压降项”
  - 风险：
    - 未来若要继续提升类型健康，仍需分批压缩

## 当前结论

### Passed

- 当前未发现新的 `P0` 可用性故障
- 当前未发现新的“常见日常任务会卡住不动”的问题
- installed command、repo wrapper、`-p`、交互启动、高频工具链当前都可用

### Overall Judgment

- 当前项目已经达到“可直接使用”的状态
- 本轮体检里识别出的活跃 `P2` 已经收口
- 本轮活跃 `P3` 也已完成收口
- 当前剩余问题主要是已建账的长期类型技术债
- 当前更像是：
  - `用户可用性：已过线`
  - `质量门禁：已补到和当前主链一致`
  - `结构纯度：active crate 树已收干净，类型债进入长期压降`

## Recommendation

建议后续优先顺序：

1. 收 `P3`：
   - `@ts-nocheck` 分批压缩
2. 做下一轮专项：
   - 交互主 runtime
   - 复杂 fallback
   - slash/session 深层路径
3. 归档资产治理：
   - 若后续确认 donor 归档已无参考价值，可再统一迁出或删除 `rust/archive/kcode-donor`
