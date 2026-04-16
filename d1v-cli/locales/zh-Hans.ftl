# D1V CLI — 简体中文翻译

## 身份验证
auth-email-prompt = 邮箱:
auth-code-sent = 验证码已发送至 { $email }
auth-code-prompt = 验证码:
auth-password-prompt = 密码:
auth-token-prompt = 令牌:
auth-token-empty = 令牌不能为空。
auth-login-success = 登录成功！
auth-logout-success = 已退出登录。
auth-not-logged-in = 未登录。
auth-status-logged-in = 已登录
auth-status-not-logged-in = 未登录
auth-status-expired = 令牌已过期
auth-status-label-user = 用户:
auth-status-label-expires = 过期时间:
warn-token-expiring = 令牌将在 { $duration }后过期，请运行 `d1v auth login` 刷新。
auth-relogin-prompt = 是否重新登录？
auth-relogin-success = 重新认证成功！

## 调试
debug-label-version = 版本:
debug-label-user-agent = 用户代理:
debug-label-locale = 语言:
debug-label-features = 特性:
debug-label-config = 配置:
debug-label-log-dir = 日志目录:
debug-label-base-url = 接口地址:
debug-label-token = 令牌:
debug-unknown = 未知
debug-features-none = 无
debug-token-found = { $source }
debug-token-expires-in = { $duration }后过期
debug-token-expired = 已过期

## CLI 错误
error-not-logged-in = 未登录
hint-not-logged-in = 请运行 `d1v auth login` 进行认证。
error-token-expired = 令牌已过期
hint-token-expired = 请运行 `d1v auth login` 重新认证。
error-network = 网络错误
error-timeout = 请求超时
error-connection-failed = 无法连接到服务器
hint-network = 请检查网络连接后重试。
hint-timeout = 请求超时，请稍后重试。
hint-connection = 请检查服务器地址和网络连接，运行 `d1v debug` 查看当前配置。
error-http-status = 服务端响应异常
error-invalid-response = 响应数据格式错误
error-invalid-url = 服务器地址无效
error-server-validation = 服务端验证失败

## API 错误码
api-error-password-not-set = 未设置密码
api-error-unknown = 服务端错误 { $code } ({ $message })
api-error-unknown-code = 服务端错误 { $code }

hint-config = 请检查配置文件 ~/.d1v/config.toml。
hint-token-storage = 请尝试运行 `d1v auth login` 重新认证。
canceled = 已取消。

## 令牌
error-no-token-store = 没有可用的令牌存储
error-keyring-unavailable = 钥匙串不可用
error-keyring-save = 保存至钥匙串失败

## 配置
error-no-home-dir = 无法确定主目录
error-read-config = 读取配置文件失败
error-write-config = 写入配置文件失败
error-parse-config = 解析配置文件失败
error-serialize-config = 序列化配置失败

## 校验
validation-email-required = 请输入邮箱地址
validation-email-invalid = 邮箱地址格式无效
validation-code-required = 请输入验证码
validation-code-length = 验证码必须为 6 位数字
validation-code-digit = 验证码只能包含数字
validation-url-invalid = 链接格式无效

## 用户
user-info-updated = 用户信息已更新。

## 密码
password-new-prompt = 新密码:
password-confirm-prompt = 确认密码:
password-mismatch = 密码不匹配。
password-empty = 密码不能为空。
password-set-success = 密码已设置。
password-forgot-sent = 密码重置邮件已发送至 { $email }。
password-reset-success = 密码重置成功。

## 邮箱
email-code-sent = 验证码已发送至 { $email }。
email-bind-success = 邮箱绑定成功。
email-change-success = 邮箱更换成功。

## 邀请与引导
invitation-accepted = 邀请已接受。
onboard-success = 引导已标记为完成。

## 确认
confirm-yes = 是
confirm-no = 否
confirm-invalid = 请输入 y 或 n。

## 时间
duration-days-hours = { $days } 天 { $hours } 小时
duration-hours-minutes = { $hours } 小时 { $minutes } 分钟
duration-minutes = { $minutes } 分钟
