# jwt_auth_security_requirement

- 状态: draft
- 日期: 2026-03-23
- 版本: v2.0

## 1. 适用范围与目标

1. 本文仅定义业务规则,用于以下能力:
   - 邮箱密码登录并签发 JWT 访问凭证
   - 刷新 JWT 访问凭证
   - 防止暴力破解
2. 本文不包含实现细节,不限定语言、框架、存储或部署方式。

## 2. 术语

1. `access_token`: 访问凭证,用于访问受保护资源,有效期固定 2 小时。
2. `refresh_token`: 刷新凭证,用于换取新访问凭证,不直接用于业务资源访问。
3. `主体标识`: 账号唯一标识,本文使用邮箱作为登录主体。
4. `风控维度`:
   - 账号维度(`email`)
   - 来源维度(`ip`)
   - 联合维度(`email+ip`)

## 3. 全局业务规则

### 3.1 字段来源优先级(多源选择规则)

1. 同名字段在多个来源同时出现时,按以下优先级取值:
   - `query` > `body` > `header` > `default`
2. 若高优先级来源存在但值非法,按非法处理,不回退到低优先级来源。
3. 优先级规则适用于登录输入、刷新输入和风控辅助字段读取。

### 3.2 标准化规则

1. 邮箱在判定前需标准化:
   - 去除首尾空白
   - 转小写
2. 空字符串视为缺失值。
3. 所有时间字段统一使用 Unix 时间戳(秒)。

### 3.3 统一失败行为

1. 失败响应统一结构: `name`、`message`、`data`。
2. 同一失败类型的 `message` 文案必须一致。
3. 认证失败(账号不存在/密码错误)统一为同一失败类型,不暴露具体原因。
4. 命中频率限制时需返回 `retry_after_seconds`(秒)。

## 4. 登录业务规则

### 4.1 触发条件与判定规则

1. 触发条件: 客户端提交登录请求。
2. 判定前置:
   - 必填字段校验通过
   - 未命中风控锁定
3. 主体判定:
   - 依据标准化邮箱定位账号
   - 账号状态必须为 `active`
4. 密码判定:
   - 密码校验通过则登录成功
   - 密码校验失败则登录失败并触发失败计数

### 4.2 输入到输出转换规则

1. 输入: `email`、`password`、可选来源信息(`ip`,`user_agent`)。
2. 转换:
   - 标准化邮箱
   - 执行风控判定
   - 进行账号与密码判定
3. 成功输出:
   - `access_token`
   - `access_token_expires_in`(固定 `7200`)
   - `refresh_token`
   - `refresh_token_expires_in`
   - `token_type`
4. 失败输出:
   - 认证失败输出 `auth_failed`
   - 风控限制输出 `rate_limited` 且带 `retry_after_seconds`
   - 参数非法输出 `invalid_params`

### 4.3 边界与例外

1. 邮箱大小写或首尾空白差异不影响账号识别结果。
2. 账号不存在与密码错误返回一致,不允许区分。
3. 账号状态为 `disabled` 时,无论密码是否正确均登录失败。
4. 在锁定期内,登录流程不进入密码判定阶段。

### 4.4 约束

1. `email`: 必填,类型为字符串,需满足邮箱格式。
2. `password`: 必填,类型为字符串,不能为空。
3. `ip`: 可选,类型为字符串;缺失时使用系统可获取来源值。
4. `user_agent`: 可选,类型为字符串。

## 5. 刷新业务规则

### 5.1 触发条件与判定规则

1. 触发条件: 客户端提交刷新请求。
2. 判定规则:
   - 刷新凭证存在
   - 刷新凭证未过期
   - 刷新凭证未被撤销
   - 刷新凭证与会话主体一致
   - 主体状态为 `active`

### 5.2 输入到输出转换规则

1. 输入: `refresh_token`。
2. 转换:
   - 校验刷新凭证有效性
   - 校验主体可用性
   - 执行凭证轮换(旧刷新凭证失效,生成新凭证对)
3. 成功输出:
   - 新 `access_token`
   - 新 `access_token_expires_in`(固定 `7200`)
   - 新 `refresh_token`
   - 新 `refresh_token_expires_in`
   - `token_type`
4. 失败输出:
   - `refresh_token_invalid`
   - `auth_failed`(主体不可用时)

### 5.3 边界与例外

1. 已失效刷新凭证重复使用必须失败。
2. 并发刷新同一凭证时,仅允许一次成功。
3. 主体被禁用后,刷新必须失败。

### 5.4 约束

1. `refresh_token`: 必填,类型为字符串。
2. 刷新成功后,旧刷新凭证必须立即不可再次用于刷新。

## 6. 防暴力破解业务规则

### 6.1 触发条件与判定规则

1. 触发条件: 发生登录尝试。
2. 风控维度:
   - `email`
   - `ip`
   - `email+ip`
3. 时间窗口: 10 分钟滑动窗口。
4. 阈值与锁定:
   - `email+ip` 连续失败 >= 5 次,锁定 15 分钟
   - `email` 累计失败 >= 10 次,锁定 30 分钟
   - `ip` 累计失败 >= 30 次,锁定 10 分钟
5. 判定顺序:
   - 先判定是否处于锁定中
   - 再决定是否允许进入账号密码判定

### 6.2 输入到输出转换规则

1. 输入: 登录尝试结果 + 风控维度值(`email`,`ip`)。
2. 转换:
   - 登录失败: 增加对应维度失败计数,并按阈值更新锁定状态
   - 登录成功: 清理 `email+ip` 维度短期失败计数;其他维度按衰减策略处理
3. 输出:
   - 未命中锁定: 允许继续认证流程
   - 命中锁定: 返回 `rate_limited` + `retry_after_seconds`

### 6.3 边界与例外

1. 未提供邮箱或邮箱非法时,仅按 `ip` 维度计数。
2. 同一来源短时高频失败必须可触发锁定。
3. 锁定到期后可恢复尝试。

### 6.4 约束

1. 风控对外不披露命中具体维度和阈值细节。
2. 风控返回文案必须统一。
3. `retry_after_seconds` 必须为非负整数(秒)。

## 7. 数据结构定义

### 7.1 登录输入结构 `LoginInput`

1. `email`: `string`, 必填
2. `password`: `string`, 必填
3. `ip`: `string`, 可选
4. `user_agent`: `string`, 可选

### 7.2 登录输出结构 `LoginOutput`

1. `access_token`: `string`, 必填
2. `access_token_expires_in`: `int64`, 必填,固定 `7200`
3. `refresh_token`: `string`, 必填
4. `refresh_token_expires_in`: `int64`, 必填
5. `token_type`: `string`, 必填

### 7.3 刷新输入结构 `RefreshInput`

1. `refresh_token`: `string`, 必填

### 7.4 刷新输出结构 `RefreshOutput`

1. `access_token`: `string`, 必填
2. `access_token_expires_in`: `int64`, 必填,固定 `7200`
3. `refresh_token`: `string`, 必填
4. `refresh_token_expires_in`: `int64`, 必填
5. `token_type`: `string`, 必填

### 7.5 失败输出结构 `ErrorOutput`

1. `name`: `string`, 必填
2. `message`: `string`, 必填
3. `data`: `object`, 必填

### 7.6 失败扩展数据结构

1. `RateLimitedData`
   - `retry_after_seconds`: `int64`, 必填
2. `DefaultErrorData`
   - 空对象,必填

### 7.7 账号业务实体 `User`

1. `id`: `string` 或 `int64`, 必填
2. `email`: `string`, 必填
3. `password_hash`: `string`, 必填
4. `status`: `enum(active, disabled)`, 必填
5. `created_at`: `int64`, 必填
6. `updated_at`: `int64`, 必填

### 7.8 刷新凭证业务实体 `RefreshTokenRecord`

1. `id`: `string` 或 `int64`, 必填
2. `user_id`: `string` 或 `int64`, 必填
3. `token_fingerprint`: `string`, 必填
4. `issued_at`: `int64`, 必填
5. `expires_at`: `int64`, 必填
6. `revoked_at`: `int64`, 可选
7. `created_ip`: `string`, 可选
8. `created_user_agent`: `string`, 可选

### 7.9 风控状态结构 `RiskState`

1. `dimension`: `enum(email, ip, email_ip)`, 必填
2. `dimension_key`: `string`, 必填
3. `fail_count`: `int32`, 必填
4. `window_start_at`: `int64`, 必填
5. `locked_until`: `int64`, 可选

## 8. 规则优先级与冲突处理

1. 风控锁定规则优先于认证规则。
2. 必填校验失败优先于业务判定。
3. 同时命中多个失败条件时,按以下优先级返回:
   - `invalid_params`
   - `rate_limited`
   - `auth_failed`
   - `refresh_token_invalid`

## 9. 非本期业务能力

1. 第三方登录。
2. 多因子认证。
3. 验证码/人机挑战。
4. 忘记密码与重置密码。

## 10. 测试网页 UI 业务规则

### 10.1 目标

1. 提供面向测试人员的网页 UI,用于手动验证登录与刷新凭证链路。
2. UI 仅用于测试与联调,不作为正式业务前台。

### 10.2 功能范围

1. 登录区域:
   - 输入 `email`、`password`
   - 触发登录并展示结果
2. 刷新区域:
   - 读取当前会话中的 `refresh_token`
   - 触发刷新并展示结果
3. 会话信息区域:
   - 展示当前 `access_token`、`refresh_token`
   - 展示 `access_token_expires_in`、`refresh_token_expires_in`
   - 展示最近一次失败信息(`name`、`message`)

### 10.3 触发条件与转换规则

1. 登录按钮触发:
   - 输入合法时发起登录流程
   - 成功后写入并展示最新凭证对
   - 失败后保留原凭证并展示失败信息
2. 刷新按钮触发:
   - 无可用 `refresh_token` 时直接提示不可执行
   - 有可用 `refresh_token` 时发起刷新流程
   - 成功后覆盖为新凭证对
   - 失败后不更新现有凭证,仅展示失败信息
3. 清空按钮触发:
   - 清空页面内会话凭证与错误信息

### 10.4 约束与边界

1. UI 不展示密码回显,不记录明文密码历史。
2. UI 仅显示必要测试信息,不展示与业务无关的内部字段。
3. 当失败类型为 `rate_limited` 时,必须展示 `retry_after_seconds`。
4. UI 支持“复制 token”能力,便于后续人工测试 `me` 等受保护能力。

## 11. 暴力破解测试业务规则

### 11.1 测试目标

1. 验证防暴力破解规则在业务层面的有效性与一致性。
2. 验证系统在高频无效凭证请求下,可稳定返回预期失败类型。

### 11.2 测试场景与触发条件

1. 场景 A: 登录口令暴力尝试
   - 使用同一 `email+ip` 在 10 分钟内连续提交错误密码
   - 触发条件: 失败次数达到阈值
2. 场景 B: 单 IP 多账号尝试
   - 同一 `ip` 对多个邮箱进行错误登录尝试
   - 触发条件: `ip` 维度失败次数达到阈值
3. 场景 C: 无效访问凭证高频请求
   - 反复提交无效或伪造 `access_token` 访问受保护能力
   - 触发条件: 请求频率达到限制阈值
4. 场景 D: 失效刷新凭证重复使用
   - 对同一已失效 `refresh_token` 重复刷新
   - 触发条件: 复用行为被识别

### 11.3 输入到输出转换规则

1. 当风控未命中阈值:
   - 按常规失败类型返回(如 `auth_failed`、`token_invalid`)
2. 当命中阈值:
   - 返回 `rate_limited`
   - `data` 中返回 `retry_after_seconds`
3. 锁定期内重复请求:
   - 直接返回 `rate_limited`
   - 不进入后续主体判定或密码判定
4. 锁定到期后:
   - 恢复正常判定流程

### 11.4 验证通过标准

1. 各维度在达到阈值后都能触发锁定。
2. 锁定期间所有同维度请求均返回一致失败类型与统一文案。
3. 返回的 `retry_after_seconds` 为非负整数且随时间递减。
4. 锁定到期后可恢复尝试,且行为符合正常流程。
5. 不因测试导致系统异常中断或出现不一致状态。

### 11.5 测试约束

1. 测试仅使用测试账号与测试环境。
2. 不允许使用真实用户数据进行暴力测试。
3. 测试过程必须可追踪,能够按请求标识回溯关键失败事件。
