# VK Tauri - Быстрый старт

## Установка зависимостей

### Linux (Debian/Ubuntu)

```bash
# Tauri dependencies
sudo apt install \
    libwebkit2gtk-4.1-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev

# GStreamer для видео/аудио
sudo apt install \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-libav \
    gstreamer1.0-vaapi

# Для Nvidia (опционально)
sudo apt install \
    nvidia-vaapi-driver \
    vdpau-driver-all
```

### Arch Linux

```bash
sudo pacman -S \
    webkit2gtk-4.1 \
    base-devel \
    openssl \
    libayatana-appindicator \
    librsvg \
    gst-plugins-good \
    gst-plugins-bad \
    gst-libav \
    gstreamer-vaapi
```

## Запуск

### Режим разработки

```bash
# Из корня проекта vk_tui/
cd vk-tauri

# Первый запуск - установить Node.js зависимости
cd ui && npm install && cd ..

# Запустить в dev mode (с hot-reload)
cargo tauri dev
```

### Production build

```bash
cd vk-tauri
cargo tauri build

# Бинарник будет в target/release/vk-tauri
./target/release/vk-tauri
```

## Проверка hardware acceleration (Nvidia)

```bash
# Проверить VA-API
vainfo

# Проверить VDPAU
vdpauinfo

# Если не работает - настроить
export WEBKIT_DISABLE_COMPOSITING_MODE=0
export LIBVA_DRIVER_NAME=nvidia
cargo tauri dev
```

## Структура проекта

```
vk-tauri/
├── src/              # Rust backend
│   ├── main.rs       # Entry point
│   ├── state.rs      # App state + LongPoll
│   └── commands.rs   # Tauri commands (RPC API)
│
├── ui/               # Svelte frontend
│   └── src/
│       ├── App.svelte
│       ├── components/
│       │   ├── AuthView.svelte      # Аутентификация
│       │   ├── MainView.svelte      # Главный экран
│       │   ├── ChatList.svelte      # Список чатов
│       │   ├── MessageView.svelte   # Просмотр сообщений
│       │   ├── Message.svelte       # Отдельное сообщение
│       │   └── MessageInput.svelte  # Ввод сообщений
│       └── main.js
│
└── tauri.conf.json   # Конфигурация Tauri
```

## Архитектура

### Backend (Rust)

```
User действие в UI
    ↓
Tauri RPC call (invoke)
    ↓
Tauri Command (commands.rs)
    ↓
vk-core AsyncCommand
    ↓
CommandExecutor → VK API
    ↓
CoreEvent
    ↓
poll_events() → Frontend
```

### Frontend (Svelte)

```javascript
// Вызов Rust функций
await invoke('load_messages', { peerId: 123, offset: 0 });

// Получение событий
setInterval(async () => {
  const events = await invoke('poll_events');
  events.forEach(handleEvent);
}, 200);
```

## Возможности

- ✅ OAuth аутентификация через браузер
- ✅ Список чатов с онлайн-статусом и непрочитанными
- ✅ Просмотр сообщений с временем и статусом прочтения
- ✅ Отправка сообщений
- ✅ Ответ на сообщения (Reply)
- ✅ **Изображения** (автозагрузка)
- ✅ **Видео** (встроенный плеер с HW acceleration)
- ✅ **Аудио** (встроенный плеер)
- ✅ Real-time обновления через LongPoll
- ✅ Индикация набора текста

## Troubleshooting

### Video не воспроизводится

```bash
# Проверить GStreamer
gst-inspect-1.0 | grep vaapi
gst-inspect-1.0 | grep libav

# Переустановить плагины
sudo apt reinstall gstreamer1.0-libav gstreamer1.0-plugins-bad
```

### High CPU usage

```bash
# Включить hardware acceleration
export WEBKIT_DISABLE_COMPOSITING_MODE=0
export WEBKIT_FORCE_COMPOSITING_MODE=1
```

### Аутентификация не работает

1. Проверьте, что открывается браузер
2. Авторизуйтесь в VK
3. Скопируйте **полный URL** из адресной строки (начинается с `https://oauth.vk.com/blank.html#access_token=...`)
4. Вставьте в поле ввода приложения

## Сравнение с Iced GUI

| Функция | Iced | Tauri |
|---------|------|-------|
| Binary size | 15-20 MB | 8-12 MB |
| RAM usage | 100-150 MB | 150-200 MB |
| Video playback | ❌ | ✅ HW accelerated |
| Audio playback | ❌ | ✅ |
| Stickers (WebP) | ❌ | ✅ |
| GIF animations | ❌ | ✅ |
| Development speed | Медленнее | Быстрее (hot-reload) |

## Лицензия

MIT OR Apache-2.0
