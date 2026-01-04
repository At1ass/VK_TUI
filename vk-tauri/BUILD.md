# VK Messenger - Build Instructions

## 📦 Supported Platforms

- ✅ **Linux** (AppImage, .deb, .rpm)
- ✅ **Windows** (.msi, .exe NSIS installer)
- ✅ **Android** (.apk, .aab)

---

## 🏗️ Local Build

### Prerequisites

**All platforms:**
- Rust (install via https://rustup.rs/)
- Node.js 18+ (install via https://nodejs.org/)

**Linux:**
```bash
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

**Windows:**
- Visual Studio Build Tools
- WebView2 Runtime (usually pre-installed on Windows 11)

**Android:**
- Android Studio with SDK
- Android NDK 25.2.9519653
- Java 17

---

## 🚀 Build Commands

### Linux

```bash
cd vk-tauri

# Build AppImage (recommended - universal format)
cargo tauri build --bundles appimage

# Build .deb package (Debian/Ubuntu)
cargo tauri build --bundles deb

# Build .rpm package (Fedora/RHEL)
cargo tauri build --bundles rpm

# Build all formats
cargo tauri build
```

**Output:**
- `target/release/bundle/appimage/` - AppImage files
- `target/release/bundle/deb/` - .deb packages
- `target/release/bundle/rpm/` - .rpm packages

### Windows

```powershell
cd vk-tauri

# Build MSI installer
cargo tauri build

# Build NSIS installer
cargo tauri build --bundles nsis
```

**Output:**
- `target\release\bundle\msi\` - .msi installer
- `target\release\bundle\nsis\` - .exe NSIS installer

### Android

```bash
cd vk-tauri

# Initialize Android project (first time only)
cargo tauri android init

# Install Android targets (first time only)
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add i686-linux-android
rustup target add x86_64-linux-android

# Build APK for development/testing
cargo tauri android build --apk

# Build AAB for Google Play Store
cargo tauri android build --aab

# Run on connected device/emulator
cargo tauri android dev
```

**Output:**
- `gen/android/app/build/outputs/apk/` - .apk files
- `gen/android/app/build/outputs/bundle/` - .aab bundles

---

## 🤖 Automated Builds (GitHub Actions)

The project includes GitHub Actions workflow that automatically builds for all platforms on tag push.

### Trigger a build:

```bash
# Create and push a version tag
git tag v0.1.0
git push origin v0.1.0
```

This will:
1. Build Linux AppImage and .deb
2. Build Windows .msi and .exe
3. Build Android .apk
4. Create GitHub Release with all binaries

### Manual trigger:

Go to GitHub Actions → Build Release → Run workflow

---

## 📱 Android Signing (for release)

To sign Android builds for distribution:

1. Generate keystore:
```bash
keytool -genkey -v -keystore vk-messenger.keystore -alias vk-messenger -keyalg RSA -keysize 2048 -validity 10000
```

2. Configure in `vk-tauri/gen/android/key.properties`:
```properties
storePassword=YOUR_STORE_PASSWORD
keyPassword=YOUR_KEY_PASSWORD
keyAlias=vk-messenger
storeFile=../../vk-messenger.keystore
```

3. Build signed APK:
```bash
cargo tauri android build --apk --release
```

---

## 🔍 Troubleshooting

### Linux: "Cannot find webkit2gtk"
```bash
sudo apt install libwebkit2gtk-4.1-dev
```

### Windows: "Cannot find WebView2"
Download and install: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

### Android: "NDK not found"
```bash
sdkmanager "ndk;25.2.9519653"
export NDK_HOME=$ANDROID_HOME/ndk/25.2.9519653
```

### Build is slow
Tauri builds can take 5-10 minutes on first run. Subsequent builds will be faster due to Rust incremental compilation.

---

## 📦 Distribution

### Linux
- **AppImage**: Works on all distributions, no installation needed
- **.deb**: For Debian/Ubuntu - `sudo dpkg -i vk-messenger_*.deb`
- **.rpm**: For Fedora/RHEL - `sudo rpm -i vk-messenger-*.rpm`

### Windows
- **.msi**: Standard Windows installer
- **.exe**: NSIS installer with custom UI

### Android
- **.apk**: For direct installation (enable "Unknown sources")
- **.aab**: For Google Play Store submission

---

## 🎯 Quick Start

**Linux users:**
```bash
cd vk-tauri
cargo tauri build --bundles appimage
./target/release/bundle/appimage/vk-messenger_*_amd64.AppImage
```

**Windows users:**
```powershell
cd vk-tauri
cargo tauri build
# Run the generated .msi installer
```

**Android users:**
```bash
cd vk-tauri
cargo tauri android init
cargo tauri android build --apk
# Install the APK on your device
```
