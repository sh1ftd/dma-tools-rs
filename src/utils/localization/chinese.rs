use super::TextKey;

pub fn get_text(key: TextKey) -> &'static str {
    match key {
        TextKey::OperationLog => "操作日志",
        TextKey::Contact => "联系方式:",
        TextKey::CopyTelegram => "复制 Telegram 链接",
        TextKey::CopyWeChat => "复制微信 ID",
        TextKey::CopyDiscord => "复制 Discord ID",
        TextKey::TelegramLink => "Telegram 链接",
        TextKey::WeChatID => "微信 ID",
        TextKey::DiscordID => "Discord ID",
        TextKey::Copied => "已复制：\n{}",
        TextKey::CheckingFiles => "正在检查文件...",
        TextKey::MissingFiles => "缺失文件",
        TextKey::FileCheckSuccess => "所有文件验证成功！",
        TextKey::SystemCheck => "系统检查",
        TextKey::WelcomeMessage => "欢迎使用 {} 工具",
        TextKey::CheckingItem => "正在检查：{}",
        TextKey::CountdownMessage => "将在 {} 秒后自动继续...",
        TextKey::ExitButton => "退出",
        TextKey::MissingFilesWarning => "警告：在缺少必要文件的情况下继续可能会导致错误",
        TextKey::GroupExecutables => "可执行文件",
        TextKey::GroupLibraries => "库文件",
        TextKey::GroupBitstreams => "比特流文件",
        TextKey::GroupConfigs => "配置文件",
        TextKey::GroupOther => "其他文件",
        TextKey::ContinueAnyway => "强制继续",
        TextKey::Rescan => "重新扫描",
        TextKey::SelectOperation => "选择操作",
        TextKey::FlashFirmware => "烧录固件",
        TextKey::ReadDna => "读取板卡 DNA",
        TextKey::SelectFirmware => "选择固件文件",
        TextKey::ScanningFirmware => "正在扫描固件文件...",
        TextKey::SelectOption => "选择选项",
        TextKey::Flash => "烧录",
        TextKey::Read => "读取",
        TextKey::Back => "返回",
        TextKey::FlashingInProgress => "正在烧录中...",
        TextKey::ReadingDnaInProgress => "正在读取 DNA...",
        TextKey::Success => "操作成功！",
        TextKey::Failed => "操作失败！",
        TextKey::TryAgain => "重试",
        TextKey::ReturnToMenu => "返回主菜单",
        TextKey::DownloadHere => "下载链接",
        TextKey::UpdateAvailable => "有可用更新",
        // Firmware Selection
        TextKey::NoFirmwareFound => "当前目录未找到固件文件",
        TextKey::PlaceFirmwareHere => "请将 .bin 固件文件放入应用程序目录",
        TextKey::AutoScanning => "每 3 秒自动扫描",
        TextKey::AutoRefreshing => "自动刷新中",
        TextKey::PerformCleanup => "执行清理",
        TextKey::CleanupDescription => "(烧录成功后删除目标 .bin 文件)",
        TextKey::Continue => "继续",
        TextKey::SelectFirmwareToContinue => "请选择固件文件以继续",
        TextKey::FlashFirmwareDesc => "烧录固件到您的设备",
        TextKey::ReadDnaDesc => "获取设备的唯一标识符 (DNA)",
        // Flashing Options
        TextKey::SelectFlashingOption => "选择烧录选项",
        TextKey::SelectDnaReadOption => "选择 DNA 读取选项",
        TextKey::Ch347Options => "CH347 选项",
        TextKey::Rs232Options => "RS232 选项",

        TextKey::Ch347_35T_Label => "CH347 - 35T",
        TextKey::Ch347_35T_Desc => "适用于使用 CH347 接口的 35T 板卡",
        TextKey::Ch347_75T_Label => "CH347 - 75T",
        TextKey::Ch347_75T_Desc => "适用于使用 CH347 接口的 75T 板卡",
        TextKey::Ch347_100T_Label => "CH347 - 100T",
        TextKey::Ch347_100T_Desc => "适用于使用 CH347 接口的 100T 板卡",

        TextKey::Rs232_35T_Label => "RS232 - 35T",
        TextKey::Rs232_35T_Desc => "适用于使用 RS232 接口的 35T 板卡",
        TextKey::Rs232_75T_Label => "RS232 - 75T",
        TextKey::Rs232_75T_Desc => "适用于使用 RS232 接口的 75T 板卡",
        TextKey::Rs232_100T_Label => "RS232 - 100T",
        TextKey::Rs232_100T_Desc => "适用于使用 RS232 接口的 100T 板卡",

        TextKey::Dna_Ch347_Label => "CH347 - DNA 读取: 35T, 75T, 100T",
        TextKey::Dna_Ch347_Desc => "使用 CH347 接口从 35T, 75T 或 100T 读取 DNA",
        TextKey::Dna_Rs232_35T_Label => "RS232 - DNA 读取: 35T",
        TextKey::Dna_Rs232_35T_Desc => "使用 RS232 接口从 35T 板卡读取 DNA",
        TextKey::Dna_Rs232_75T_Label => "RS232 - DNA 读取: 75T",
        TextKey::Dna_Rs232_75T_Desc => "使用 RS232 接口从 75T 板卡读取 DNA",
        TextKey::Dna_Rs232_100T_Label => "RS232 - DNA 读取: 100T",
        TextKey::Dna_Rs232_100T_Desc => "使用 RS232 接口从 100T 板卡读取 DNA",

        // Log View
        TextKey::ClearLog => "清除日志",

        // Result Extras
        TextKey::OperationTook => "操作耗时",
        TextKey::NoteFewerSectors => "注意：操作完成但少于 10 个扇区。请手动验证或重试。",
        TextKey::NoteVerifySuccess => "注意：无法验证完全成功，但未检测到错误。请手动验证或重试。",
        TextKey::ErrorDetails => "错误详情",

        // Progress
        TextKey::Initializing => "初始化中...",
        TextKey::StartingOperation => "正在开始操作...",
        TextKey::WritingImage => "正在写入镜像到闪存...",
        TextKey::ProbingFlash => "正在探测闪存...",
        TextKey::ResettingFpga => "正在重置并暂停 FPGA...",
        TextKey::LoadingBitstream => "正在加载比特流...",
        TextKey::InitJtag => "正在初始化 JTAG 接口...",
        TextKey::Verifying => "正在测试和验证...",
        TextKey::WritingSector => "正在写入扇区",
        TextKey::ReadingDeviceDna => "正在读取设备 DNA",
        TextKey::PleaseWaitDna => "请稍候，我们正在从您的设备获取唯一标识符。",
        TextKey::DnaTakesSeconds => "这通常需要几秒钟即可完成。",
        TextKey::FlashingFirmware => "正在烧录固件",
        TextKey::PleaseWaitFlash => "请稍候，固件正在写入您的设备。",
        TextKey::FlashTakesMinutes => "这通常需要 1-2 分钟完成。",
        TextKey::FlashFailImmediate => "如果过程立即完成，则可能已失败。",
        TextKey::TechnicalInfo => "技术信息",
        TextKey::InterfaceLabel => "接口:",
        TextKey::OperationTypeLabel => "操作类型:",
        TextKey::TargetDeviceLabel => "目标设备:",

        // Result
        TextKey::DnaReadSuccess => "DNA 读取成功！",
        TextKey::DnaReadFailed => "DNA 读取失败",
        TextKey::DnaReadUnexpected => "DNA 读取状态异常",
        TextKey::DeviceDnaHeader => "设备 DNA",
        TextKey::ClickToCopy => "点击复制",
        TextKey::FlashingSuccess => "烧录成功！",
        TextKey::FlashingFailed => "烧录失败",
        TextKey::FlashingFailedConnection => "烧录失败 - 连接问题",
        TextKey::FlashingResultUnknown => "烧录结果未知",
        TextKey::NextSteps => "后续步骤",
        TextKey::NextStepsList => {
            "1. 重启两台电脑\n2. 按照指南中的后续步骤操作\n   - 在主机上安装固件驱动程序\n   - 将电缆更换到 DATA 端口\n   - 使用提供的软件和激活码进行激活\n   - DNA 锁定的固件版本无需激活"
        }
        TextKey::Exit => "退出",
        TextKey::MainMenu => "主菜单",
        TextKey::TryAgainButton => "重试",

        // Detailed Result Messages
        TextKey::DnaReadUnexpectedMsg => {
            "操作完成，但无法确认 DNA 值。\n这可能表明 DNA 提取过程存在问题。\n请检查日志输出以获取详细信息。"
        }
        TextKey::DnaReadFailedPrefix => "从设备读取 DNA 失败：",
        TextKey::OperationInProgress => "操作进行中：",
        TextKey::DnaStatusUnknownMsg => "DNA 读取操作状态未知。\n请检查日志以获取详细信息。",
        TextKey::ClickToCopyTooltip => "点击复制 RAW, HEX 和 Verilog DNA 值",
        TextKey::FlashingFailedConnectionMsg => {
            "检测到的正常扇区写入不足： {} / {} 扇区。\n\n这表明存在硬件连接问题。设备虽然可访问，但数据未正确传输。\n\n尝试：\n1. 使用不同的 USB 端口\n2. 检查线缆连接\n3. 确保设备供电正常\n4. 尝试更换 USB 线缆"
        }
        TextKey::FlashingResultUnknownMsg => {
            "烧录过程已完成，但在日志中未找到扇区写入信息。\n\n1. 确认您选择了正确的板卡类型\n2. 确认已安装正确的 USB 驱动程序并连接到 JTAG 端口。\n3. 尝试更换 USB 线缆和/或端口\n4. 确保设备已正确安装在 PCIE 插槽中。"
        }
        TextKey::UnexpectedStateMsg => "不应达到此状态。请报告此错误。",
        TextKey::FlashingFailedPrefix => "向设备烧录固件失败：",
        TextKey::FlashStatusUnknownMsg => "烧录操作状态未知。\n请检查日志以获取详细信息或重试。",
        // DNA Backend & Status
        TextKey::DnaInvalidOption => "DNA 读取选项无效",
        TextKey::DnaCommandFailed => "执行 DNA 读取命令失败",
        TextKey::DnaFileNotFound => "尝试 {} 次后仍未找到 DNA 输出文件",
        TextKey::DnaExtractFailed => "提取 DNA 失败：{}",
        TextKey::DnaFileReadError => "读取位于 {} 的 DNA 输出文件失败：{}",
        TextKey::DnaInfoNotFound => "在输出文件中未找到 DNA 信息",
        TextKey::DnaWaitingStart => "等待开始读取 DNA...",
        TextKey::DnaRetrieving => "正在获取设备 DNA...",
        TextKey::DnaReadSuccessStatus => "DNA 读取成功！",
        TextKey::DnaOperationCompleted => "操作已完成（非 DNA）",
        TextKey::DnaReadFailedStatus => "DNA 读取失败：{}",
    }
}
