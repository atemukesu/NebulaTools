#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    ChineseSimplified,
    English,
}

impl Language {
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::ChineseSimplified => "简体中文",
            Language::English => "English",
        }
    }
}

pub fn tr(lang: Language, key: &str) -> &'static str {
    match lang {
        Language::ChineseSimplified => match key {
            "window_title" => "NebulaTools - NBL 格式调试器",
            "file" => "文件",
            "import" => "导入 .nbl",
            "export" => "导出 (未实现)",
            "metadata" => "元数据",
            "playback" => "播放控制",
            "total_frames" => "总帧数",
            "textures" => "纹理列表",
            "version" => "版本",
            "fps" => "目标 FPS",
            "particle_count" => "当前粒子数",
            "error" => "错误",
            "play" => "播放",
            "pause" => "暂停",
            "stop" => "停止",
            "frame" => "帧",
            "language" => "语言",
            "inspector" => "数据检视器",
            "preview_3d" => "3D 预览",
            "preview_hint" => "左键旋转，滚动缩放",
            "bbox" => "包围盒",
            "yes" => "是",
            "no" => "否",
            "has_alpha" => "颜色包含 Alpha",
            "has_size" => "包含大小信息",
            "duration" => "时长",
            "keyframe_count" => "关键帧数",
            _ => "Unknown",
        },
        Language::English => match key {
            "window_title" => "NebulaTools - NBL Format Debugger",
            "file" => "File",
            "import" => "Import .nbl",
            "export" => "Export (WIP)",
            "metadata" => "Metadata",
            "playback" => "Playback",
            "total_frames" => "Total Frames",
            "textures" => "Textures",
            "version" => "Version",
            "fps" => "Target FPS",
            "particle_count" => "Active Particles",
            "error" => "Error",
            "play" => "Play",
            "pause" => "Pause",
            "stop" => "Stop",
            "frame" => "Frame",
            "language" => "Language",
            "inspector" => "Inspector",
            "preview_3d" => "3D Preview",
            "preview_hint" => "Drag to rotate, Scroll to zoom",
            "bbox" => "Bounding Box",
            "yes" => "Yes",
            "no" => "No",
            "has_alpha" => "Has Alpha",
            "has_size" => "Has Size",
            "duration" => "Duration",
            "keyframe_count" => "Keyframes",
            _ => "Unknown",
        },
    }
}
