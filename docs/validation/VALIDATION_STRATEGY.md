# Validation Strategy

## 验证对象

ORG-X 验证的是早期生产方式判断是否被后来的企业事实支持，而不是短期股价表现。

```text
Stage 2 / Stage 3 detected at T0
        -> 6 months
        -> 12 months
        -> 24 months
```

每个时间点检查：

- productivity divergence 是否出现并持续；
- economic capture 是否发生；
- production model 是否持续，而非短期实验；
- competitor imitation 与 industry diffusion 是否出现。

对最高层 `ReferenceModel` 的验收是 fail-closed 的四族证据门：组织重写、生产系统重写、至少两个结果周期、至少两个独立扩散来源及命名同行。缺少结果或扩散只能保留为 `Candidate`；来源不可用、未知或只有公司自述不能被当作反证，也不能升级资格。反证检索是否执行、结果如何以及仍缺哪些证明，必须在评估和周报中分开记录。

## 基线

验证必须保存 T0 的 Stage、Evidence IDs、假设、反证、缺失证明和同行基线。后续观察以相同口径比较 Revenue/Employee、Operating Income/Employee、FCF/Employee、增长与 headcount 变化，并记录来源质量。

## 次级结果

股票收益可以作为 secondary outcome 记录，但不能替代生产率和扩散验证，也不能改变 ORG-X 的非交易边界。

## 当前实现边界

`features::validation` 现在提供一个只保存验证证据的 bounded context：

- T0 基线保留公司标识、调用方提供的 Stage 文本、Evidence IDs、假设、反证、缺失证明和同行基线；
- 后续观察固定为 6 个月、12 个月和 24 个月，并保留五个验证维度、opaque metric value/unit、source quality 和 Evidence references；
- 空值、重复 Evidence ID、重复 metric name、重复 horizon 和重复公司记录在边界拒绝，拒绝不会修改已有记录；
- `ValidationEvaluator` 只报告缺少哪些 horizon 以及记录是否 complete，不计算 Stage、score、ranking、threshold、经济显著性或投资结论。

这个 bounded context 目前是内存 store 和纯 application/domain 边界，不接入 Weekly Radar runtime，不调度真实的 6/12/24 个月任务，也不包含外部 Provider 数据。权威的 S&P 500/Nasdaq 100 universe、生产运行 receipt、runtime judgment-chain integration 和 source-host 安全策略仍需独立证据或产品决策。
