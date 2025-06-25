# translated by @JustKanade
# Language info 
language-name = 简体中文

# Tabs
music = 音乐
sounds = 音效
images = 图片
rbxm-files = RBXM文件
ktx-files = KTX纹理
settings = 设置
about = 关于
logs = 日志

# Buttons
button-reset-sql-db = Reset SQL Database # TODO: Translate
button-change-sql-db = Change SQL Database # TODO: Translate
button-delete-this-dir = 删除此目录 <Del>
button-extract-type = 提取此类型的所有文件 <F3>
button-refresh = 刷新 <F5>
button-clear-cache = 清理Roblox缓存 <Del>
button-extract-all = 提取全部 <F3>
button-change-cache-dir = 更改缓存目录
button-reset-cache-dir = 重置缓存目录
button-finish = 完成
button-yes = 是
button-no = 否
button-rename = 重命名 <F2>
button-search = 搜索 <Ctrl+F>
button-swap = 交换资源 <F4>
button-copy-logs = 复制日志到剪贴板
button-export-logs = 导出日志到文件
button-copy = 复制 <Ctrl+D>
button-open = 打开 <Return>
button-extract-file = 提取 <Ctrl+E>
button-display-image-preview = 显示图片预览
button-disable-display-image-preview = 停止显示图片预览
input-preview-size = 预览大小

# Confirmations
confirmation-generic-confirmation-title = 确认
confirmation-delete-confirmation-title = 删除文件
confirmation-delete-confirmation-description = 您确定要删除此目录中的文件吗？
confirmation-filter-confirmation-title = 文件仍在过滤中
confirmation-filter-confirmation-description = 您确定要在程序过滤时提取文件吗？这将导致提取不完整
confirmation-clear-cache-title = 清理Roblox缓存
confirmation-clear-cache-description = 您确定要清理缓存吗？文件将在客户端加载时重新生成
confirmation-custom-directory-title = 选择不同的目录
confirmation-custom-directory-description = 您想要选择不同的目录吗？
confirmation-ban-warning-title = 潜在风险警告
confirmation-ban-warning-description = 在游戏中编辑资源可能导致客户端表现异常

# Errors
error-invalid-database-description = Please make sure the path you provided is an SQLite Database # TODO: Translate
error-invalid-database-title = Invalid database! # TODO: Translate
error-sql-detection-title = Database detection failed! # TODO: Translate
error-sql-detection-description = Database detection failed! Is the client installed and you ran it at least once? # TODO: Translate
no-files = 没有要列出的文件
error-directory-detection-title = 目录检测失败
error-directory-detection-description = Roblox是否已安装且 您至少运行过一次？
error-temporary-directory-title = 创建临时目录失败！
error-temporary-directory-description = 错误：创建临时目录失败！您是否有临时文件夹的读写权限？如果此错误继续出现，请尝试以管理员身份运行
error-invalid-directory-title = 无效目录！
error-invalid-directory-description = 请确保您提供的路径是一个目录
generic-error-critical = 严重错误

# Headings
actions = 操作
updates = 更新
language-settings = 语言设置
new-updates = 有新更新可用
contributors = 贡献者
dependencies = 依赖项
behavior = 可选项

# Checkboxes
check-for-updates = 检查更新
automatically-install-updates = 自动安装更新
use-alias = 导出您重命名的文件名
use-topbar-buttons = 启用工具栏
refresh-before-extract = 提取前刷新文件列表
download-development-build = 使用开发版本以提前获得功能
checkbox-hide-user-logs = 从日志中隐藏用户名

# Descriptions
custom-sql-db-description = If you want to access a different cache, change your SQL Database below, you can set it back to default with the other button. This is different from your installation folder. # TODO: Translate
clear-cache-description = 如果从目录列出文件耗时太长，您可以清理Roblox缓存来帮助解决，客户端将在需要时重新生成这些文件
extract-all-description = 此按钮将把所有资源提取到文件夹
custom-cache-dir-description = 如果您想访问不同的缓存，可以在下方更改
use-alias-description = 使用您在此程序中重命名时使用的名称
swap-choose-file = 双击文件进行替换
swap-with = 双击文件与"{ $asset }"替换
logs-description = 日志显示程序的运行状态，如果发生任何错误，它们会在这里显示
copy-choose-file = 双击文件进行复制
overwrite-with = 双击文件以用"{ $asset }"覆盖

# Statuses
idling = 空闲中
deleting-files = 正在删除文件 ({ $item }/{ $total })
reading-files = 正在读取文件 ({ $item }/{ $total })
extracting-files = 正在提取文件 ({ $item }/{ $total })
filtering-files = 正在过滤文件 ({ $item }/{ $total })
all-extracted = 所有文件已提取
stage = 阶段 { $stage }/{ $max }: { $status }
swapped = 已将 { $item_a } 与 { $item_b } 替换
copied = 已用 { $item_a } 覆盖 { $item_b }

# Misc
sql-database = SQL Database: { $path } # TODO: Translate
no-function = （尚未功能化）
version = 版本: v{ $version } (编译于 { $date })
cache-directory = 缓存目录: { $directory }
welcome = 欢迎
download-update-question = 您想要下载更新吗？
update-changelog = 下方为更新日志
support-sponsor = 赞助
support-project-donate = 捐赠
setting-below-restart-required = 注意：更改此设置需要重启程序
failed-deleting-file = 错误：删除失败 ({ $item }/{ $total })
failed-opening-file = 错误：打开文件失败
error-extracting-file = 错误：提取失败，原因：{ $error }
error-check-logs = 错误，请检查日志
failed-not-file = 错误 '{ $file }' 不是文件
