# Tauri + React + Steam Template

A modern, production-ready starter template for building desktop games for **Steam** using **Tauri v2**, **React 18**, **TypeScript**, and **SWC**.

This template comes pre-configured with essential Steamworks features including **Achievements**, **Leaderboards**, and **Cloud Saves**, paired with high-performance desktop window management provided by Tauri.

---

## 🚀 Features

- **Tauri v2 Desktop Engine**: Lightweight, secure, and fast native desktop app wrapper powered by Rust.
- **React 18 + TypeScript (SWC)**: Type-safe component UI development using Vite with `@vitejs/plugin-react-swc` for ultra-fast compilation.
- **Steamworks Integration**: Ready-to-use hooks and commands for Steam Achievements, Leaderboards, and Cloud Save.
- **Built-in Sass/SCSS Support**: Pre-configured styling support for `.scss` and `.sass` files.
- **License Compliance Tooling**: Integrated `license-checker` script to generate third-party license reports for Steam releases.
- **Tauri Plugins Pre-installed**:
  - `@tauri-apps/plugin-fs` (Local file system operations)
  - `@tauri-apps/plugin-process` (Process control and application restart)
  - `@tauri-apps/plugin-store` (Persistent key-value storage)
- **UI Agnostic**: Minimal core UI, allowing seamless setup for Tailwind CSS, Bootstrap, or custom styles.

---

## 🛠️ Prerequisites

Before getting started, make sure your development environment has the following installed:

1. **Node.js** (v18 or higher recommended) & `npm`
2. **Rust** (`rustc` and `cargo` via [rustup.rs](https://rustup.rs/))
3. **C++ Build Tools**:
   - **Windows**: Visual Studio C++ Build Tools (select _Desktop development with C++_)
   - **macOS**: Xcode Command Line Tools (`xcode-select --install`)
   - **Linux**: Build essentials and WebKitGTK packages (`build-essential`, `libssl-dev`, `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`)
4. **Steam Client**: Steam must be running on your machine for Steamworks API integration during development.

---

## 📦 Installation & Setup

1. **Clone the repository**:

   ```bash
   git clone https://github.com/your-username/tauri-react-steam-template.git
   cd tauri-react-steam-template
   ```

2. **Install Node.js dependencies**:

   ```bash
   npm install
   ```

3. **Configure Steam App ID**:
   - Open `src-tauri/steam_appid.txt`.
   - By default, it uses `480` (Steam's official _Spacewar_ test App ID).
   - Replace `480` with your actual game's Steam App ID provided in your Steamworks Dashboard.

---

## 💻 Development Workflow

To run the application in development mode with live reload:

```bash
npm run tauri:dev
```

This launches both the Vite local development server and the Tauri application window.

### Available Scripts

- `npm run dev`: Runs Vite dev server only (web browser mode).
- `npm run build`: Compiles TypeScript and builds production web assets.
- `npm run tauri:dev`: Starts Tauri in development mode with HMR.
- `npm run tauri:build`: Bundles the full game binary for release.
- `npm run check-licenses`: Displays a summary of all third-party package licenses.
- `npm run export-licenses`: Exports third-party package licenses to a CSV file for Steam compliance.

---

## 🎮 Steamworks Integration

The project includes a custom IPC integration layer with Steam via Rust.

### Rust Backend Configuration (`src-tauri/Cargo.toml`)

The Rust side leverages the `steamworks` crate to bind native Steam APIs:

```toml
[dependencies]
tauri = { version = "2.2.0" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
steamworks = "0.11"
```

### Unlocking Achievements Example

```typescript
import { invoke } from "@tauri-apps/api/core";

// Unlock a Steam Achievement
async function unlockAchievement(achievementId: string) {
  try {
    await invoke("unlock_achievement", { name: achievementId });
    console.log(`Achievement ${achievementId} unlocked!`);
  } catch (error) {
    console.error("Failed to unlock achievement:", error);
  }
}
```

---

## 🎨 Styling & UI Framework Integration

### Sass / SCSS Support

Sass is included out of the box (`sass`). Import `.scss` or `.sass` files directly into your components:

```tsx
import "./styles/game-ui.scss";
```

### Optional: Adding Tailwind CSS

To install Tailwind CSS:

1. Install Tailwind and its dependencies:
   ```bash
   npm install -D tailwindcss postcss autoprefixer
   npx tailwindcss init -p
   ```
2. Configure `tailwind.config.js`:
   ```javascript
   /** @type {import('tailwindcss').Config} */
   export default {
     content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
     theme: {
       extend: {},
     },
     plugins: [],
   };
   ```
3. Add directives to `./src/index.css`:
   ```css
   @tailwind base;
   @tailwind components;
   @tailwind utilities;
   ```

### Optional: Adding Bootstrap / React-Bootstrap

To install Bootstrap:

1. Install packages:
   ```bash
   npm install bootstrap react-bootstrap
   ```
2. Import Bootstrap CSS in `./src/main.tsx`:
   ```typescript
   import "bootstrap/dist/css/bootstrap.min.css";
   ```

---

## ⚖️ License Compliance for Steam

Steam requires games to comply with open-source licensing. Use the built-in scripts powered by `license-checker`:

```bash
# View license summary in console
npm run check-licenses

# Output license report to a CSV file
npm run export-licenses
```

---

## 📄 License

This project is licensed under the **MIT License**. Feel free to use it for personal or commercial Steam game developments.
