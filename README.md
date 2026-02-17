# NebulaTools

NebulaTools 是一个高性能的 .nbl 粒子动画编辑与预览工具。
NebulaTools is a high-performance tool for editing and previewing .nbl particle animations.
NebulaTools は、高性能な .nbl パーティクルアニメーション編集・プレビューツールです。

---

### 核心功能 | Core Features | 主要機能

**1. 实时 3D 预览 | Real-time 3D Preview | リアルタイム 3D プレビュー**
- 支持高性能粒子渲染与平滑的相机控制。
- Supports high-performance particle rendering and smooth camera controls.
- 高性能なパーティクルレンダリングとスムーズなカメラ操作をサポート。

**2. 流式编辑工具 | Streaming Edit Tools | ストリーミング編集ツール**
- 提供动画速度调整、粒子大小缩放、颜色调整及坐标变换等功能。
- Provides features for animation speed adjustment, particle scaling, color adjustment, and coordinate transformation.
- アニメーション速度調整、サイズ変更、色調整、座標変換などの機能を提供。
- 采用流式处理技术，支持超大文件的快速导出。
- Utilizes streaming processing to support fast export of large files.
- ストリーミング処理技術を採用し、大容量ファイルの高速エクスポートを実現。

**3. NBL 压缩 (实验性) | NBL Compression (Experimental) | NBL 圧縮 (実験的)**
- 使用 I/P 帧增量算法重新编码，显著减小文件体积。
- Re-encodes using I/P frame delta algorithms to significantly reduce file size.
- I/P フレーム差分アルゴリズムを使用して再エンコードし、ファイルサイズを大幅に削減。

**4. Particleex 命令编译器 | Particleex Command Compiler | Particleex コマンドコンパイラ**
- 支持使用数学表达式动态控制粒子的运动轨迹与属性。
- Supports dynamic control of particle trajectories and attributes using mathematical expressions.
- 数式を使用して、パーティクルの運動軌道や属性を動的に制御可能。

**5. 粒子创建器 | Particle Creator | パーティクル作成器**
- 内置多种预设（烟花、喷泉、螺旋等），快速生成自定义粒子动画。
- Built-in presets (fireworks, fountains, spirals, etc.) for quick custom particle animation generation.
- 花火、噴水、螺旋などのプリセットを内蔵し、カスタムアニメーションを素早く生成。

---

### 技术栈 | Tech Stack | 技術スタック

- **Rust**: 核心逻辑与性能保障。
- **Rust**: Core logic and performance assurance.
- **Rust**: コアロジックとパフォーマンスの保証。
- **egui / glow**: 现代化跨平台 UI 与 OpenGL 渲染。
- **egui / glow**: Modern cross-platform UI and OpenGL rendering.
- **egui / glow**: モダンなクロスプラットフォーム UI と OpenGL レンダリング。

---

### 如何运行 | How to Run | 実行方法

确保已安装 Rust 环境，在项目根目录下运行：
Ensure the Rust environment is installed, then run in the project root:
Rust 環境がインストールされていることを確認し、プロジェクトのルートディレクトリで実行してください：

```powershell
cargo run --release
```
