#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
=============================================================================
NebulaTools 自动化构建与发布脚本 (Python 版)
=============================================================================
功能:
  1. 自动构建 Windows x64 MSVC 版本
  2. 自动打包 exe 与 assets 并生成 Markdown 更新日志
  3. 可选：上传构建产物到 GitHub Releases [Y/n]

使用方法:
  python build_and_release.py [版本号]
=============================================================================
"""

import glob
import hashlib
import os
import re
import shutil
import subprocess
import sys
import zipfile
from datetime import datetime


PROJECT_NAME = "NebulaTools"
APP_NAME = "nebula_tools"
ASSETS_DIR = "assets"
BUILD_ROOT = "build"
RELEASES_DIR = os.path.join(BUILD_ROOT, "releases")
TARGET = "x86_64-pc-windows-msvc"
DISPLAY_NAME = "windows-x64-msvc"
EXE_NAME = f"{APP_NAME}.exe"
DIST_DIR = os.path.join(RELEASES_DIR, DISPLAY_NAME)

VERSION = "unknown"
BASE_VERSION = "unknown"
GIT_HASH = "unknown"
BUILD_DATE = "unknown"
ZIP_NAME = ""
ZIP_PATH = ""


class Colors:
    RED = "\033[0;31m"
    GREEN = "\033[0;32m"
    YELLOW = "\033[1;33m"
    BLUE = "\033[0;34m"
    NC = "\033[0m"


def log_info(msg):
    print(f"{Colors.BLUE}[INFO]{Colors.NC} {msg}")


def log_success(msg):
    print(f"{Colors.GREEN}[SUCCESS]{Colors.NC} {msg}")


def log_warning(msg):
    print(f"{Colors.YELLOW}[WARNING]{Colors.NC} {msg}")


def log_error(msg):
    print(f"{Colors.RED}[ERROR]{Colors.NC} {msg}")


def ask_yes_no(prompt: str, default_yes=True) -> bool:
    choices = "[Y/n]" if default_yes else "[y/N]"
    reply = input(f"\n{Colors.YELLOW}>>> {prompt} {choices}:{Colors.NC} ").strip().lower()
    if not reply:
        return default_yes
    return reply.startswith("y")


def run_cmd(cmd_list, capture=False, check=True, cwd=None):
    try:
        if capture:
            result = subprocess.run(
                cmd_list,
                cwd=cwd,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                encoding="utf-8",
                errors="replace",
                check=check,
            )
            return result.stdout.strip() if result.stdout else ""

        process = subprocess.Popen(
            cmd_list,
            cwd=cwd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            bufsize=1,
        )
        assert process.stdout is not None
        for line in process.stdout:
            print(line, end="")
        process.wait()
        if check and process.returncode != 0:
            log_error(f"命令执行失败: {' '.join(cmd_list)}")
            sys.exit(1)
        return ""
    except subprocess.CalledProcessError as exc:
        if check:
            log_error(f"命令执行失败: {' '.join(cmd_list)}")
            if capture and exc.stderr:
                log_error(f"错误输出: {exc.stderr.strip()}")
            sys.exit(1)
        return ""


def load_env_file():
    env_file = ".env"
    if not os.path.isfile(env_file):
        log_info("未找到 .env 文件，将使用系统环境变量")
        return

    log_info("检测到 .env 文件，正在加载...")
    with open(env_file, "r", encoding="utf-8") as file:
        for line in file:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if "=" not in line:
                continue
            key, value = line.split("=", 1)
            os.environ[key.strip()] = value.strip().strip('"\'')
            log_info(f"已加载：{key.strip()}")
    log_success(".env 文件加载完成")


def check_environment():
    log_info("检查环境依赖...")
    load_env_file()

    if not shutil.which("cargo"):
        log_error("Cargo 未安装或不在 PATH 中")
        sys.exit(1)

    if not shutil.which("git"):
        log_error("Git 未安装或不在 PATH 中")
        sys.exit(1)

    cargo_version = run_cmd(["cargo", "--version"], capture=True, check=False)
    if cargo_version:
        log_info(f"Cargo 版本：{cargo_version}")

    rustc_version = run_cmd(["rustc", "--version"], capture=True, check=False)
    if rustc_version:
        log_info(f"Rust 版本：{rustc_version}")

    log_success("环境检查完成")


def get_version_info():
    global VERSION, BASE_VERSION, GIT_HASH, BUILD_DATE, ZIP_NAME, ZIP_PATH

    with open("Cargo.toml", "r", encoding="utf-8") as file:
        for line in file:
            match = re.match(r'^version = "(.+)"', line)
            if match:
                BASE_VERSION = match.group(1)
                break

    if len(sys.argv) > 1:
        VERSION = sys.argv[1]
        log_info(f"使用指定的版本号：{VERSION}")
    else:
        VERSION = BASE_VERSION
        log_info(f"使用默认版本号：{VERSION}")

    GIT_HASH = run_cmd(["git", "rev-parse", "--short", "HEAD"], capture=True)
    BUILD_DATE = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
    ZIP_NAME = f"{PROJECT_NAME}.{VERSION}.{DISPLAY_NAME}.zip"
    ZIP_PATH = os.path.join(RELEASES_DIR, ZIP_NAME)

    log_info(f"构建日期：{BUILD_DATE}")
    log_info(f"Git 提交：{GIT_HASH}")


def collect_changelog_input():
    changelog_file = os.path.join(BUILD_ROOT, "manual_changelog.md")
    os.makedirs(BUILD_ROOT, exist_ok=True)

    if not ask_yes_no("是否要手动输入更新描述？", default_yes=False):
        return

    print("\n请输入本版本的主要更新内容（每行一条，直接按回车结束输入）:")
    print("例如：")
    print("  - 优化了导出流程")
    print("  - 修复了界面卡顿问题\n")

    updates = []
    while True:
        line = input("> ").strip()
        if not line:
            break
        updates.append(line)

    if not updates:
        log_info("未输入更新内容，将仅使用 Git 提交记录")
        return

    with open(changelog_file, "w", encoding="utf-8") as file:
        file.write("## 主要更新\n\n")
        for update in updates:
            file.write(f"{update}\n")

    log_success(f"已记录 {len(updates)} 条更新内容")


def clean_release_dirs():
    if os.path.exists(RELEASES_DIR):
        log_info(f"正在清理旧的发布目录: {RELEASES_DIR}")
        shutil.rmtree(RELEASES_DIR)
    os.makedirs(DIST_DIR, exist_ok=True)
    log_success("发布目录已清空，准备构建新版本")


def build_project():
    log_info("==========================================")
    log_info(f"开始构建 {PROJECT_NAME} {DISPLAY_NAME} 版本")
    log_info("==========================================")

    run_cmd(["cargo", "build", "--release", "--target", TARGET])

    src_exe = os.path.join("target", TARGET, "release", EXE_NAME)
    dst_exe = os.path.join(DIST_DIR, EXE_NAME)
    if not os.path.isfile(src_exe):
        log_error(f"构建成功后仍未找到可执行文件: {src_exe}")
        sys.exit(1)

    log_info(f"复制程序: {src_exe} -> {dst_exe}")
    shutil.copy2(src_exe, dst_exe)

    if os.path.isdir(ASSETS_DIR):
        dst_assets = os.path.join(DIST_DIR, ASSETS_DIR)
        log_info(f"复制资源: {ASSETS_DIR} -> {dst_assets}")
        shutil.copytree(ASSETS_DIR, dst_assets)

    log_success("构建产物整理完成")


def create_zip_package():
    log_info(f"正在打包: {ZIP_NAME}")
    with zipfile.ZipFile(ZIP_PATH, "w", zipfile.ZIP_DEFLATED) as archive:
        for root, _, files in os.walk(DIST_DIR):
            for file_name in files:
                file_path = os.path.join(root, file_name)
                arc_name = os.path.relpath(file_path, RELEASES_DIR)
                archive.write(file_path, arc_name)

    if not os.path.isfile(ZIP_PATH):
        log_error(f"打包失败，未生成文件: {ZIP_PATH}")
        sys.exit(1)

    log_success(f"打包完成: {ZIP_PATH}")


def get_file_hash(filepath, algo="md5"):
    hasher = hashlib.new(algo)
    with open(filepath, "rb") as file:
        while chunk := file.read(8192):
            hasher.update(chunk)
    return hasher.hexdigest()


def get_file_size_mb(filepath):
    size_bytes = os.path.getsize(filepath)
    return f"{size_bytes / (1024 * 1024):.2f}M"


def generate_changelog():
    log_info("生成更新日志...")
    changelog_file = os.path.join(BUILD_ROOT, "changelog.md")
    manual_changelog = os.path.join(BUILD_ROOT, "manual_changelog.md")
    prev_tag = run_cmd(["git", "describe", "--tags", "--abbrev=0"], capture=True, check=False)

    with open(changelog_file, "w", encoding="utf-8") as file:
        file.write(f"# {PROJECT_NAME} v{VERSION}\n\n")
        file.write("## 发布信息\n")
        file.write(f"- **发布日期**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
        file.write(f"- **Git 提交**: {GIT_HASH}\n")
        file.write(f"- **构建目标**: {TARGET}\n")
        if prev_tag:
            file.write(f"- **上一版本**: {prev_tag}\n")
        file.write("\n")

        if os.path.isfile(manual_changelog):
            with open(manual_changelog, "r", encoding="utf-8") as manual_file:
                file.write(manual_file.read())
            file.write("\n")

        file.write("## 详细提交记录\n\n")
        if prev_tag:
            file.write(f"### 自上版本 {prev_tag} 以来的变更\n")
            commits = run_cmd(
                ["git", "log", "--pretty=format:- %s (%h)", f"{prev_tag}..HEAD"],
                capture=True,
                check=False,
            )
        else:
            file.write("### 最近提交\n")
            commits = run_cmd(["git", "log", "--pretty=format:- %s (%h)", "-20"], capture=True, check=False)
        file.write((commits if commits else "- 暂无详细记录") + "\n\n")

        if shutil.which("gh"):
            prs = run_cmd(
                [
                    "gh",
                    "pr",
                    "list",
                    "--state",
                    "merged",
                    "--limit",
                    "10",
                    "--json",
                    "number,title,author",
                    "--template",
                    "{{range .}}- #{{.number}} {{.title}} (by @{{.author.login}})\n{{end}}",
                ],
                capture=True,
                check=False,
            )
            if prs.strip():
                file.write("### 合并的 Pull Requests\n")
                file.write(prs + "\n\n")

        file.write("---\n\n")
        file.write("## 文件清单\n\n")
        file.write("本次发布包含以下文件：\n")
        file.write(f"- `{ZIP_NAME}` ({get_file_size_mb(ZIP_PATH)})\n")
        file.write(f"  - MD5: `{get_file_hash(ZIP_PATH, 'md5')}`\n")
        file.write(f"  - SHA256: `{get_file_hash(ZIP_PATH, 'sha256')}`\n")
        exe_path = os.path.join(DIST_DIR, EXE_NAME)
        if os.path.isfile(exe_path):
            file.write(f"- `{DISPLAY_NAME}/{EXE_NAME}` ({get_file_size_mb(exe_path)})\n")

    log_success(f"更新日志已生成：{changelog_file}")


def upload_to_github():
    if not ask_yes_no("是否要将构建产物发布到 GitHub Releases？", default_yes=True):
        log_info("已跳过 GitHub Releases 上传。")
        return

    gh_repo = os.environ.get("GITHUB_REPOSITORY")
    if not os.environ.get("GITHUB_TOKEN") or not gh_repo:
        log_warning("缺少 GITHUB_TOKEN 或 GITHUB_REPOSITORY 环境变量，无法上传。")
        return

    if not shutil.which("gh"):
        log_warning("未安装 GitHub CLI (gh)，跳过上传。")
        return

    log_info("==========================================")
    log_info("开始上传到 GitHub Releases")
    log_info("==========================================")

    release_tag = f"v{VERSION}"
    changelog_file = os.path.join(BUILD_ROOT, "changelog.md")
    check_exists = run_cmd(["gh", "release", "view", release_tag, "--repo", gh_repo], capture=True, check=False)
    if "title:" in check_exists:
        log_warning(f"Release {release_tag} 已存在，正在删除旧版本...")
        run_cmd(["gh", "release", "delete", release_tag, "--cleanup-tag", "--yes", "--repo", gh_repo], check=False)

    run_cmd(
        [
            "gh",
            "release",
            "create",
            release_tag,
            ZIP_PATH,
            "--repo",
            gh_repo,
            "--title",
            f"{PROJECT_NAME} {release_tag}",
            "--notes-file",
            changelog_file,
        ]
    )
    log_success("成功上传到 GitHub Releases")
    log_info(f"Release 链接：https://github.com/{gh_repo}/releases/tag/{release_tag}")


def cleanup():
    log_info("清理临时文件...")
    for temp_file in glob.glob(os.path.join(BUILD_ROOT, "manual_changelog.md")):
        try:
            os.remove(temp_file)
        except OSError:
            pass
    log_success("清理完成")


def main():
    print("==============================================")
    print("  NebulaTools 自动化构建与发布工具 (Python)")
    print("==============================================")

    os.chdir(os.path.dirname(os.path.abspath(__file__)))

    check_environment()
    get_version_info()
    collect_changelog_input()
    clean_release_dirs()
    build_project()
    create_zip_package()
    generate_changelog()

    print("\n==============================================")
    log_info("即将进入发布阶段")
    print("==============================================")
    upload_to_github()
    cleanup()

    print("\n==============================================")
    log_success("所有任务完成!")
    print("==============================================\n")
    print("构建产物汇总:")
    print(f"  - 压缩包：{os.path.abspath(ZIP_PATH)}")
    print(f"  - 目录：{os.path.abspath(DIST_DIR)}")
    print(f"  - 更新日志：{os.path.abspath(os.path.join(BUILD_ROOT, 'changelog.md'))}\n")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\n用户取消了操作。")
        sys.exit(0)
