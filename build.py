import os
import sys
import shutil
import zipfile
import subprocess
import re


def get_version():
    with open("Cargo.toml", "r", encoding="utf-8") as f:
        for line in f:
            match = re.match(r'^version = "(.+)"', line)
            if match:
                return match.group(1)
    raise RuntimeError("无法从 Cargo.toml 中获取版本号")


def run_command(cmd, cwd=None):
    print(f"执行命令: {' '.join(cmd)}")
    with subprocess.Popen(
        cmd,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    ) as proc:
        for line in proc.stdout:
            print(line, end="")
        proc.wait()
        if proc.returncode != 0:
            print(f"命令失败，退出码: {proc.returncode}")
            sys.exit(1)


def main():
    PROJECT_NAME = "NebulaTools"
    APP_NAME = "nebula_tools"
    ASSETS_DIR = "assets"
    OUTPUT_DIR = "dist"

    VERSION = get_version()
    print(f"正在构建项目: {PROJECT_NAME} v{VERSION}")

    TARGET = "x86_64-pc-windows-msvc"
    DISPLAY_NAME = "windows-x64-msvc"
    EXE_NAME = f"{APP_NAME}.exe"

    print("----------------------------------------")
    print(f"正在构建目标: {TARGET} ({DISPLAY_NAME})")

    print("执行构建...")
    run_command(["cargo", "build", "--release", "--target", TARGET])

    shutil.rmtree(OUTPUT_DIR, ignore_errors=True)
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    BUILD_DIR = os.path.join(OUTPUT_DIR, DISPLAY_NAME)
    os.makedirs(BUILD_DIR, exist_ok=True)

    src_exe = os.path.join("target", TARGET, "release", EXE_NAME)
    dst_exe = os.path.join(BUILD_DIR, EXE_NAME)
    print(f"复制程序: {src_exe} -> {dst_exe}")
    shutil.copy2(src_exe, dst_exe)

    if os.path.exists(ASSETS_DIR):
        dst_assets = os.path.join(BUILD_DIR, ASSETS_DIR)
        print(f"复制资源: {ASSETS_DIR} -> {dst_assets}")
        shutil.copytree(ASSETS_DIR, dst_assets)

    ZIP_NAME = f"{PROJECT_NAME}.{VERSION}.{DISPLAY_NAME}.zip"
    print(f"正在打包: {ZIP_NAME}")

    with zipfile.ZipFile(ZIP_NAME, "w", zipfile.ZIP_DEFLATED) as zf:
        for root, dirs, files in os.walk(BUILD_DIR):
            for file in files:
                file_path = os.path.join(root, file)
                arc_name = os.path.relpath(file_path, OUTPUT_DIR)
                zf.write(file_path, arc_name)

    shutil.rmtree(OUTPUT_DIR)

    print("----------------------------------------")
    print("构建任务已完成！")
    if os.path.exists(ZIP_NAME):
        size = os.path.getsize(ZIP_NAME) / (1024 * 1024)
        print(f"{ZIP_NAME} ({size:.2f} MB)")


if __name__ == "__main__":
    main()
