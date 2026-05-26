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
auth-status-label-user = 用户
auth-status-label-expires = 过期时间
warn-token-expiring = 令牌将在 { $duration }后过期，请运行 `d1v auth login` 刷新。
auth-relogin-prompt = 令牌已过期，是否重新登录？
auth-relogin-yes = 是，重新登录
auth-relogin-no = 否，退出
auth-relogin-success = 重新认证成功！
auth-method-prompt = 登录方式
auth-method-code = 验证码
auth-method-password = 密码
auth-method-token = 认证令牌

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
error-invalid-base-url = 服务器地址 "{ $value }" 无效（来自 { $source }）
hint-invalid-base-url-cli = 请传入有效的 URL，或省略该参数。
hint-invalid-base-url-env = 请取消 `D1V_BASE_URL` 或将其设为有效的 URL。
hint-invalid-base-url-config = 请修改 ~/.d1v/config.toml 中的 `base_url`。
error-server-validation = 服务端验证失败

## API 错误码
api-error-bad-request = 请求无效
api-error-bad-request-message = 请求无效（{ $message }）
api-error-password-not-set = 未设置密码
api-error-invalid-credentials = 邮箱或密码无效
api-error-email-required-before-password = 请先绑定邮箱再设置密码
api-error-invalid-code = 验证码无效
api-error-code-expired = 验证码已过期
api-error-code-invalid-or-expired = 验证码无效或已过期
api-error-user-not-found = 用户不存在
api-error-password-too-short = 密码太短
api-error-email-in-use = 邮箱已被使用
api-error-email-not-bound = 未绑定邮箱
api-error-invite-own-code = 不能接受自己的邀请码
api-error-invite-invalid = 邀请码无效
api-error-invite-expired = 邀请码已过期
api-error-invite-capacity = 邀请码容量已满
api-error-invite-limit = 该邀请码已达到邀请上限
api-error-invite-not-bound = 邀请码未绑定邀请人
api-error-inviter-not-found = 邀请人不存在
api-error-auth-required = 认证失败
api-error-auth-required-message = 认证失败（{ $message }）
api-error-permission-denied = 没有访问权限
api-error-permission-denied-message = 没有访问权限（{ $message }）
api-error-insufficient-privileges = 需要超级管理员账号
api-error-unknown = 服务端错误 { $code } ({ $message })
api-error-unknown-code = 服务端错误 { $code }

hint-config = 请检查配置文件 ~/.d1v/config.toml。
hint-token-storage = 请尝试运行 `d1v auth login` 重新认证。
canceled = 已取消。
interrupted = 已中断。

## 令牌
error-no-token-store = 没有可用的令牌存储
error-keyring-unavailable = 钥匙串不可用
error-keyring-load = 从钥匙串读取失败
error-keyring-save = 保存至钥匙串失败
error-keyring-delete = 从钥匙串删除失败

## 配置
error-no-home-dir = 无法确定主目录
error-read-config = 读取配置文件失败
error-write-config = 写入配置文件失败
error-parse-config = 解析配置文件失败
error-serialize-config = 序列化配置失败
error-invalid-config-value = { $key } 的值无效: { $value }
config-set-success = { $key } = { $value }
config-reset-success = 配置已重置为默认值。
config-edit-failed = 无法打开配置文件

## 校验
validation-email-required = 请输入邮箱地址
validation-email-invalid = 邮箱地址格式无效
validation-code-required = 请输入验证码
validation-code-length = 验证码必须为 6 位数字
validation-code-digit = 验证码只能包含数字
validation-url-invalid = 链接格式无效

## 用户
user-info-updated = 用户信息已更新。
user-update-field-prompt = 选择要更新的信息
user-update-field-company-name = 公司名称
user-update-field-company-website = 公司网站
user-update-field-picture = 头像 URL
user-update-field-industry = 行业
user-update-field-referral-code = 邀请码
user-label-id = ID:
user-label-slug = 标识:
user-label-email = 邮箱:
user-label-roles = 角色:
user-label-company = 公司:
user-label-website = 网站:
user-label-industry = 行业:
user-label-invite-code = 邀请码:

## 活动
activity-label-period = 时间范围:
activity-label-days = 天数:

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

## 选择
select-action-navigate = 移动
select-action-confirm = 确认
select-action-cancel = 取消
select-ctrl-c-hint = 再次按 { $key } 退出

## 时间
duration-days-hours = { $days } 天 { $hours } 小时
duration-hours-minutes = { $hours } 小时 { $minutes } 分钟
duration-minutes = { $minutes } 分钟

## 升级
upgrade-up-to-date = d1v { $version } 已是最新版本。
upgrade-available = 发现新版本：{ $current } -> { $latest }
upgrade-downloading = 正在下载 d1v { $version }...
upgrade-download-complete = 下载完成
upgrade-verifying = 正在校验发布包...
upgrade-installing = 正在安装更新...
upgrade-success = 🎉 升级成功！已安装发布版本 { $version }。

## 卸载
uninstall-success = 🗑️ 卸载成功！已从 { $path } 移除 d1v。

## 环境变量
project-required = 需要指定项目。请使用 -p 参数、设置 D1V_PROJECT_ID 环境变量、或在 d1v 工作区中执行。
env-key-not-found = 未找到键 "{ $key }"。
env-set-summary = 新建 { $created } 个，更新 { $updated } 个
env-import-summary = 新建 { $created } 个，更新 { $updated } 个，跳过 { $skipped } 个（共 { $total } 个）
env-export-saved = 已导出到 { $path }
env-import-stdin-required = 请通过管道传入 .env 内容，或使用 -i 指定文件路径。
env-sync-confirm-required = 使用 --yes 参数确认同步到 Vercel。
env-label-key = 键
env-label-value = 值
env-label-description = 描述
env-label-sensitive = 敏感
env-label-message = 消息
env-label-dev-project = 开发项目
env-label-dev-env-count = 开发环境变量数
env-label-dev-up-to-date = 开发环境已同步
env-label-prod-project = 生产项目
env-label-prod-env-count = 生产环境变量数
env-label-prod-up-to-date = 生产环境已同步
env-empty-list = 暂无环境变量。
env-yes = 是
env-no = 否
