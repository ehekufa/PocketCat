# PocketCat

Rust-порт мобильного конструктора игр Pocket-Up/Pocket Code. UI рисуется Slint (`ui/main.slint`), игровая модель/движок реализованы на Rust; Android использует Slint NativeActivity backend.

## Запуск и тесты

```sh
cargo run
cargo test
```

## Android APK

GitHub Actions собирает debug APK для `arm64-v8a`. Запустить workflow вручную можно через **Actions → Android APK → Run workflow**. APK сохраняется как артефакт `pocketcat-debug-apk`.

Локально нужны Rust stable, Android SDK/NDK, затем:

```sh
rustup target add aarch64-linux-android
cargo install cargo-apk
cargo apk build --lib --target aarch64-linux-android
```

## Перенос игрового ядра

`src/scene.rs` содержит сцены, спрайты, события и блоки: StartScene/ResumeScene, координаты, переменные, гравитация, кэширование и JSON-импорт/экспорт проекта. Сервисная граница сцены предназначена для аудио, сенсоров и системного ввода.

UI перенесён на Slint. Это продолжающийся порт: визуальные экраны/категории и базовые блоки есть, но полного паритета с исходным Pocket-Up пока нет — редактор рисования, формулы и весь каталог блоков требуют дальнейшей миграции.
