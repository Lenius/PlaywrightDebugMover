# Playwright Debug Mover

**Playwright Debug Mover** is a lightweight background utility written in Rust that automatically detects and moves Playwright debug windows to the top-right corner of the screen.

---

## ✨ Features

- ✅ Runs silently without showing a console window
- ✅ Ensures only one instance runs at a time
- ✅ Monitors for visible windows with titles starting with `"Playwright"`
- ✅ Moves matching windows to the top-right corner of the screen
- ✅ Built-in web interface to stop the program via browser (`http://localhost:5000`)

---

## 🛠 Build Instructions

### Prerequisites

- Rust (install via [https://rustup.rs](https://rustup.rs))
- Windows (tested on Windows 10/11)

### Build steps

1. Clone or download the project

```bash
git clone git@github.com:Lenius/PlaywrightDebugMover.git
cd PlaywrightDebugMover
```

2. Build the release version

```bash
cargo build --release
```

3. You will find the compiled `.exe` in:

```
target/release/PlaywrightDebugMover.exe
```

---

## 🚀 Usage

Run the program (double click or run via terminal):

```bash
target/release/PlaywrightDebugMover.exe
```

- The app will monitor for Playwright debug windows.
- Matching windows will be repositioned automatically to the top-right corner of the screen.

### Using the program

- Go to [http://localhost:5000](http://localhost:5000)
- Click the **Start** button  
  → The program will start looking for Playwright Debug window
- Click the **Stop** button  
  → The program will stop lokking for Playwright Debug window
- Click the **“Kill”** button  
  → The program will shut down cleanly
- Click the **Template** button  
  → The program generates a template file
---

## 🧪 Development tips

To clean previous builds before compiling:

```bash
cargo clean
```

To test changes quickly:

```bash
cargo run
```

To fully rebuild in release mode:

```bash
cargo build --release
```

---

## 📝 License

MIT License

---

## 🙋‍♂️ Support / Contact

Feel free to fork, open issues, or contribute pull requests.
