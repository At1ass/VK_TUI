# GNOME HIG Compliance Implementation Plan

## Цель
Привести Tauri интерфейс в полное соответствие с [GNOME Human Interface Guidelines](https://developer.gnome.org/hig/) и Libadwaita design patterns.

## Референсы
- [GNOME HIG](https://developer.gnome.org/hig/)
- [Libadwaita CSS Variables](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1.2/css-variables.html)
- [Libadwaita Adaptive Layouts](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1.5/adaptive-layouts.html)
- [Fractal messenger UI patterns](https://thisweek.gnome.org/posts/2025/08/twig-210/)

---

## Phase 1: Adwaita CSS Variables (Критично)

### 1.1 Заменить все кастомные CSS переменные на официальные Libadwaita

**Текущие проблемы в `app.css`:**
```css
/* НЕПРАВИЛЬНО - кастомные переменные */
--window-bg-color: #1e1e1e;
--view-bg-color: #242424;
--headerbar-bg-color: #2a2a2a;
/* ... и т.д. */
```

**Правильные значения из Libadwaita:**
```css
/* ПРАВИЛЬНО - официальные значения */
--window-bg-color: #222226;
--view-bg-color: #1d1d20;
--headerbar-bg-color: #2e2e32;
--headerbar-fg-color: #ffffff;
--headerbar-border-color: rgba(255, 255, 255, 0.07);
--headerbar-shade-color: rgb(0 0 6 / 36%);
--sidebar-bg-color: #2e2e32;
--sidebar-fg-color: #ffffff;
--sidebar-shade-color: rgb(0 0 6 / 25%);
--card-bg-color: rgb(255 255 255 / 8%);
--card-fg-color: #ffffff;
--accent-bg-color: #3584e4;  /* --accent-blue */
--accent-fg-color: #ffffff;
--destructive-bg-color: #c01c28;
--destructive-fg-color: #ffffff;
--success-bg-color: #26a269;
--border-color: rgba(255, 255, 255, 0.07);
--shade-color: rgb(0 0 6 / 25%);
--window-radius: 15px;
```

**Задачи:**
- [ ] Обновить `:root` в `app.css` официальными значениями
- [ ] Удалить все кастомные значения
- [ ] Добавить CSS переменные для fonts:
  ```css
  --document-font-family: "Cantarell", "Adwaita Sans", sans-serif;
  --document-font-size: 12pt;
  --monospace-font-family: "Adwaita Mono", "Source Code Pro", monospace;
  ```

---

## Phase 2: Header Bar (Критично)

### 2.1 Правильная структура header bar по HIG

**Из гайдлайнов:**
- Height: `46px` (минимум)
- Три зоны: start (left), center, title
- Кнопки в header bar - **без видимого фона** (`.flat` style)
- Title - `font-size: 14px`, `font-weight: 600`

**Текущая структура в `MainView.svelte`:**
```svelte
<!-- НЕПРАВИЛЬНО -->
<header class="header">
  <h2>Сообщения</h2>
  <div class="header-right">
    <!-- всё в одной куче -->
  </div>
</header>
```

**Правильная структура:**
```svelte
<header class="headerbar">
  <div class="headerbar-start">
    <!-- Основные действия: back, add, etc. -->
  </div>

  <div class="headerbar-center">
    <h1 class="headerbar-title">Сообщения</h1>
  </div>

  <div class="headerbar-end">
    <!-- Вторичные действия и меню -->
    <button class="button flat icon-button" on:click={onLogout}>
      <!-- Icon: system-log-out-symbolic -->
    </button>
  </div>
</header>
```

**CSS для header bar:**
```css
.headerbar {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 6px;
  padding: 0 6px;
  min-height: 46px;
  background: var(--headerbar-bg-color);
  border-bottom: 1px solid var(--headerbar-border-color);
  box-shadow: 0 1px var(--headerbar-shade-color);
}

.headerbar-start,
.headerbar-end {
  display: flex;
  align-items: center;
  gap: 6px;
}

.headerbar-center {
  display: flex;
  justify-content: center;
  align-items: center;
}

.headerbar-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--headerbar-fg-color);
}
```

**Задачи:**
- [ ] Рефакторить `MainView.svelte` header на правильную структуру
- [ ] Переместить поиск из header bar в content area или toolbar
- [ ] Кнопка logout - `.flat` стиль с иконкой
- [ ] Status индикатор - переместить в subtitle или убрать

---

## Phase 3: Sidebar & Navigation Split View

### 3.1 Sidebar sizing и структура

**Из HIG:**
- Default width: `25%` от total width
- Min width: `180sp` (~270px)
- Max width: `280sp` (~420px)
- Background: `--sidebar-bg-color`
- Border: `1px solid var(--sidebar-shade-color)`

**Текущий CSS в `ChatList.svelte`:**
```css
/* НЕПРАВИЛЬНО */
.chat-list {
  width: 280px;  /* фиксированная ширина */
  background: var(--sidebar-bg-color);
}
```

**Правильный CSS:**
```css
.sidebar-pane {
  min-width: 270px;
  max-width: 420px;
  width: 25%;
  background: var(--sidebar-bg-color);
  border-right: 1px solid var(--sidebar-shade-color);
  box-shadow: inset -1px 0 var(--sidebar-shade-color);
}
```

**Задачи:**
- [ ] Изменить `.chat-list` на `.sidebar-pane`
- [ ] Применить правильные размеры (min/max/width: 25%)
- [ ] Добавить `box-shadow` для глубины

### 3.2 AdwNavigationSplitView pattern

**Структура приложения должна быть:**
```svelte
<div class="navigation-split-view">
  <div class="sidebar-pane">
    <!-- ChatList -->
  </div>

  <div class="content-pane">
    <!-- MessageView -->
  </div>
</div>
```

**CSS:**
```css
.navigation-split-view {
  display: flex;
  width: 100%;
  height: 100vh;
  overflow: hidden;
}

.content-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--view-bg-color);
}

/* Responsive: collapse to mobile view */
@media (max-width: 600px) {
  .navigation-split-view {
    /* Stack vertically or hide sidebar */
  }
}
```

**Задачи:**
- [ ] Создать wrapper `.navigation-split-view` в `MainView.svelte`
- [ ] Разделить на `.sidebar-pane` и `.content-pane`
- [ ] Добавить responsive breakpoints

---

## Phase 4: List Patterns (Chat List)

### 4.1 Proper list row design

**Из Fractal и HIG:**
- Row height: минимум `56px`
- Padding: `12px` vertical, `12px` horizontal
- Border radius для hover/selection: `6px` (--radius-s)
- Margin между rows: `0` (слитный список)

**Текущий CSS в `ChatList.svelte`:**
```css
/* НЕПРАВИЛЬНО */
.chat-item {
  padding: 0.5rem 0.75rem;  /* ~8px 12px */
  margin: 0.1rem 0.35rem;   /* gaps between items */
}
```

**Правильный CSS:**
```css
.list-row {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 12px;
  min-height: 56px;
  background: transparent;
  border: none;
  border-radius: 0;
  transition: background 150ms ease-out;
}

.list-row:hover {
  background: var(--row-hover-bg-color);
}

.list-row.selected {
  background: var(--accent-bg-color);
  color: var(--accent-fg-color);
}

.list-row-header {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.list-row-title {
  flex: 1;
  font-size: 13px;
  font-weight: 600;
}

.list-row-subtitle {
  font-size: 11px;
  color: var(--muted-fg-color);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Badges */
.unread-badge {
  background: var(--accent-bg-color);
  color: var(--accent-fg-color);
  font-size: 10px;
  font-weight: 600;
  padding: 2px 6px;
  border-radius: 999px;
  min-width: 20px;
  text-align: center;
}
```

**Задачи:**
- [ ] Переименовать `.chat-item` → `.list-row`
- [ ] Убрать margins между items
- [ ] Изменить padding на `12px`
- [ ] Min-height: `56px`
- [ ] Selected state - `--accent-bg-color` вместо `--accent-bg-color-dim`

---

## Phase 5: Message Content Area

### 5.1 Document style class для сообщений

**Из Libadwaita 1.7:**
> The `.document` style class uses a larger document font (12pt instead of 11pt)
> and increases line height, intended for app content such as messages in chat clients.

**Текущая структура `Message.svelte`:**
```svelte
<!-- Нет .document класса -->
<div class="message-bubble">
  <p class="message-text">{message.text}</p>
</div>
```

**Правильная структура:**
```svelte
<div class="message-container">
  <div class="message-bubble" class:outgoing={message.is_outgoing}>
    <p class="message-text document">{message.text}</p>
  </div>
</div>
```

**CSS:**
```css
.document {
  font-family: var(--document-font-family);
  font-size: var(--document-font-size); /* 12pt */
  line-height: 1.6;
}

.message-bubble {
  max-width: 65%;
  padding: 8px 12px;
  border-radius: 12px;
  background: var(--card-bg-color);
  box-shadow: 0 1px 2px var(--shade-color);
}

.message-bubble.outgoing {
  background: var(--accent-bg-color);
  color: var(--accent-fg-color);
}
```

**Задачи:**
- [ ] Добавить `.document` класс к тексту сообщений
- [ ] Обновить font-size с 13px на 12pt
- [ ] Увеличить line-height до 1.6
- [ ] Использовать `--card-bg-color` для входящих
- [ ] Использовать `--accent-bg-color` для исходящих

### 5.2 Message input styling

**Правильный CSS для input:**
```css
.message-input-container {
  display: flex;
  gap: 6px;
  padding: 12px;
  background: var(--view-bg-color);
  border-top: 1px solid var(--border-color);
}

.message-input {
  flex: 1;
  background: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 8px 12px;
  font-size: 13px;
  line-height: 1.4;
  min-height: 36px;
  max-height: 120px;
  resize: vertical;
}

.message-input:focus {
  border-color: var(--accent-bg-color);
  outline: none;
  box-shadow: inset 0 0 0 1px var(--accent-bg-color);
}
```

---

## Phase 6: Buttons

### 6.1 Правильные button styles по HIG

**Из HIG:**
- Header bar buttons - **flat** (без фона)
- Primary actions - `.suggested` (accent color)
- Destructive actions - `.destructive` (red)
- Regular buttons - default style

**Текущий CSS в `app.css`:**
```css
/* Более-менее правильно, но нужны корректировки */
.button {
  min-height: 28px;  /* Маловато */
  padding: 0.35rem 0.75rem;
}
```

**Правильный CSS:**
```css
.button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 32px;
  padding: 6px 12px;
  font-size: 13px;
  font-weight: 600;
  border-radius: 6px;
  border: 1px solid transparent;
  background: var(--card-bg-color);
  color: var(--view-fg-color);
  transition: all 150ms ease-out;
}

.button:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.1);
}

.button.flat {
  background: transparent;
  border: none;
  box-shadow: none;
}

.button.flat:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.06);
}

.button.suggested {
  background: var(--accent-bg-color);
  color: var(--accent-fg-color);
  border: none;
}

.button.suggested:hover:not(:disabled) {
  background: color-mix(in srgb, var(--accent-bg-color) 85%, black);
}

.button.destructive {
  background: var(--destructive-bg-color);
  color: var(--destructive-fg-color);
  border: none;
}

.button.icon-button {
  min-width: 32px;
  padding: 6px;
}
```

**Задачи:**
- [ ] Обновить min-height с 28px на 32px
- [ ] Изменить padding на `6px 12px`
- [ ] Добавить font-weight: 600
- [ ] Улучшить hover states
- [ ] Добавить `.icon-button` класс

---

## Phase 7: Search UI

### 7.1 Переместить поиск из header bar

**Из HIG:**
> Search должен быть отдельным UI элементом, не в header bar основного окна.

**Варианты:**
1. **Toolbar под header bar** (как в Files)
2. **Search button в header → открывает search view**
3. **Ctrl+F открывает overlay search**

**Рекомендация:** Вариант 2 - кнопка поиска в header-end, открывает отдельную панель

**Структура:**
```svelte
<header class="headerbar">
  <div class="headerbar-end">
    <button class="button flat icon-button" on:click={toggleSearch}>
      <!-- Icon: system-search-symbolic -->
    </button>
  </div>
</header>

{#if searchActive}
  <div class="search-bar">
    <input type="search" placeholder="Поиск сообщений..." />
    <label>
      <input type="checkbox" bind:checked={searchInChat} />
      В текущем чате
    </label>
  </div>
{/if}
```

**CSS:**
```css
.search-bar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 6px 12px;
  background: var(--headerbar-bg-color);
  border-bottom: 1px solid var(--border-color);
}

.search-bar input[type="search"] {
  flex: 1;
  background: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 6px 12px;
  min-height: 32px;
}
```

**Задачи:**
- [ ] Убрать поиск из header bar
- [ ] Добавить search button (`.flat.icon-button`)
- [ ] Создать `.search-bar` под header bar
- [ ] Добавить keyboard shortcut Ctrl+F

---

## Phase 8: Modals & Dialogs

### 8.1 Правильные dialog styles

**Из HIG:**
- Dialog background: `--dialog-bg-color`
- Border radius: `12px` (--radius-l)
- Padding: `24px`
- Actions: right-aligned, gap `12px`

**Текущий CSS в `MessageView.svelte`:**
```css
.modal {
  padding: 1rem;  /* 16px - маловато */
  border-radius: var(--radius-l);
}
```

**Правильный CSS:**
```css
.dialog {
  background: var(--dialog-bg-color);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 24px;
  min-width: 360px;
  max-width: 480px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
}

.dialog-header {
  margin-bottom: 18px;
}

.dialog-title {
  font-size: 15px;
  font-weight: 700;
}

.dialog-content {
  margin-bottom: 24px;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}
```

**Задачи:**
- [ ] Переименовать `.modal` → `.dialog`
- [ ] Увеличить padding до 24px
- [ ] Использовать `--dialog-bg-color`
- [ ] Структурировать на header/content/actions

---

## Phase 9: Responsive & Adaptive

### 9.1 Breakpoints по HIG

**Breakpoints:**
- Mobile: `< 600px` - sidebar collapse
- Tablet: `600px - 900px` - sidebar visible, narrow
- Desktop: `> 900px` - full layout

**CSS:**
```css
/* Mobile: collapsed sidebar */
@media (max-width: 600px) {
  .sidebar-pane {
    position: fixed;
    left: -100%;
    width: 80%;
    max-width: 320px;
    height: 100vh;
    z-index: 100;
    transition: left 200ms ease-out;
  }

  .sidebar-pane.revealed {
    left: 0;
    box-shadow: 2px 0 8px rgba(0, 0, 0, 0.3);
  }

  .content-pane {
    width: 100%;
  }
}

/* Tablet */
@media (min-width: 600px) and (max-width: 900px) {
  .sidebar-pane {
    min-width: 200px;
    width: 30%;
  }
}
```

**Задачи:**
- [ ] Добавить breakpoints
- [ ] Mobile: sidebar collapse + reveal button
- [ ] Tablet: узкий sidebar
- [ ] Desktop: полный layout

---

## Phase 10: Polish & Details

### 10.1 Мелкие улучшения

**Задачи:**
- [ ] Scrollbars - правильный стиль (8px width, overlay)
- [ ] Transitions - 150ms ease-out
- [ ] Focus outlines - `2px solid var(--accent-bg-color)`
- [ ] Tooltips для всех кнопок без текста
- [ ] Loading spinners - правильный размер (32px)
- [ ] Context menu - правильный border-radius и shadow

---

## Чеклист финальной проверки

- [ ] Все CSS переменные - из Libadwaita
- [ ] Header bar - 46px height, три зоны
- [ ] Sidebar - 25% width, 270-420px range
- [ ] List rows - 56px min height, без margins
- [ ] Messages - `.document` класс, 12pt font
- [ ] Buttons - 32px min height, правильные styles
- [ ] Search - вне header bar
- [ ] Dialogs - 24px padding, правильная структура
- [ ] Responsive - breakpoints работают
- [ ] Все интерактивные элементы имеют tooltips

---

## Источники

- [GNOME HIG](https://developer.gnome.org/hig/)
- [Libadwaita CSS Variables](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1.2/css-variables.html)
- [Libadwaita 1.7 Features](https://nyaa.place/blog/libadwaita-1-7/)
- [AdwNavigationSplitView](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1.2/class.NavigationSplitView.html)
- [Fractal messenger updates](https://thisweek.gnome.org/posts/2025/08/twig-210/)
