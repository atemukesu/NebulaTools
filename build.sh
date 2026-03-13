#!/bin/bash

# 设置退出即停止
set -e

# --- 配置区 ---
PROJECT_NAME="NebulaTools"
APP_NAME="nebula_tools"  # Cargo.toml 中的 package name
ASSETS_DIR="assets"
OUTPUT_DIR="dist"

# 从 Cargo.toml 中提取版本号
VERSION=$(grep "^version =" Cargo.toml | head -n 1 | cut -d '"' -f 2)
if [ -z "$VERSION" ]; then
    echo "错误: 无法从 Cargo.toml 中获取版本号"
    exit 1
fi

echo "正在构建项目: $PROJECT_NAME v$VERSION"

# 定义构建目标及其别名
# 格式: "target_triple:display_name:executable_name"
TARGETS=(
    "x86_64-unknown-linux-gnu:linux-x64:$APP_NAME"
    "x86_64-pc-windows-gnu:windows-x64:$APP_NAME.exe"
    "i686-pc-windows-gnu:windows-i686:$APP_NAME.exe"
)

# 创建输出目录
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# 遍历目标进行构建
for TARGET_INFO in "${TARGETS[@]}"; do
    IFS=":" read -r TRIPLE DISPLAY_NAME EXE_NAME <<< "$TARGET_INFO"
    
    echo "----------------------------------------"
    echo "正在构建目标: $TRIPLE ($DISPLAY_NAME)"
    
    # 检查 target 是否已安装，未安装则尝试安装
    if ! rustup target list --installed | grep -q "$TRIPLE"; then
        echo "正在安装 target: $TRIPLE..."
        rustup target add "$TRIPLE"
    fi

    # 执行构建
    cargo build --release --target "$TRIPLE"

    # 准备打包目录
    BUILD_DIR="$OUTPUT_DIR/$DISPLAY_NAME"
    mkdir -p "$BUILD_DIR"

    # 复制二进制程序
    cp "target/$TRIPLE/release/$EXE_NAME" "$BUILD_DIR/"

    # 复制资源文件夹 (如果存在)
    if [ -d "$ASSETS_DIR" ]; then
        cp -r "$ASSETS_DIR" "$BUILD_DIR/"
    fi

    # 打包成 ZIP
    ZIP_NAME="${PROJECT_NAME}.${VERSION}.${DISPLAY_NAME}.zip"
    echo "正在打包: $ZIP_NAME"
    
    # 进入目录打包以避免包含冗余路径
    (cd "$OUTPUT_DIR" && 7z a -tzip "../$ZIP_NAME" "$DISPLAY_NAME")

    echo "完成: $ZIP_NAME"
done

# 清理临时构建目录
rm -rf "$OUTPUT_DIR"

echo "----------------------------------------"
echo "所有构建任务已完成！"
ls -lh *.zip
