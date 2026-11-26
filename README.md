# PocketCat

The visual block palette is based on the block categories and catalog in [upsouls/Pocket-Up](https://github.com/upsouls/Pocket-Up) (upstream revision `5b2ff2fd6c41f28196c277729cf5d15ca8323998`). PocketCat's catalog is metadata-only: displaying a block does not imply that its runtime behavior has been implemented.

Rust-порт мобильного конструктора игр Pocket-Up/Pocket Code. UI рисуется Slint (`ui/main.slint`), игровая модель/движок реализованы на Rust; Android использует Slint NativeActivity backend.

## Запуск и тесты

```sh
cargo run
cargo test
```

## Android APK

GitHub Actions собирает оптимизированный APK для `arm64-v8a` в release-профиле. Запустить workflow вручную можно через **Actions → Android APK → Run workflow**. APK сохраняется как артефакт `pocketcat-debug-apk`. Для каждой CI-сборки используется временный ключ подписи, поэтому перед установкой новой сборки удалите предыдущую версию PocketCat.

Локально нужны Rust stable, Android SDK/NDK, затем:

```sh
rustup target add aarch64-linux-android
cargo install cargo-apk
cargo apk build --lib --target aarch64-linux-android
```

## Перенос игрового ядра

`src/scene.rs` содержит сцены, спрайты, события и блоки: StartScene/ResumeScene, координаты, переменные, гравитация, кэширование и JSON-импорт/экспорт проекта. Сервисная граница сцены предназначена для аудио, сенсоров и системного ввода.

UI перенесён на Slint. Это продолжающийся порт: визуальные экраны/категории и базовые блоки есть, но полного паритета с исходным Pocket-Up пока нет — редактор рисования, формулы и весь каталог блоков требуют дальнейшей миграции.
