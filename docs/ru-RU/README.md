[![Скачать для Windows](https://img.shields.io/github/downloads/AeEn123/RoExtract/latest/RoExtract-windows.exe?label=Скачать&color=blue&style=for-the-badge)](https://github.com/AeEn123/RoExtract/releases/latest/download/RoExtract-windows.exe)
[![Скачать для Linux](https://img.shields.io/github/downloads/AeEn123/RoExtract/latest/RoExtract-linux?label=Скачать&style=for-the-badge)](https://github.com/AeEn123/RoExtract/releases/latest/download/RoExtract-linux)
[![Веб-сайт](https://img.shields.io/badge/Веб--сайт-red?logo=googlechrome&style=for-the-badge)](https://aeen123.github.io/RoExtract/)

[![Build and Release](https://github.com/AeEn123/RoExtract/actions/workflows/build-and-release.yml/badge.svg)](https://github.com/AeEn123/RoExtract/actions/workflows/build-and-release.yml)
[![Discord-приглашение](https://img.shields.io/discord/470242481582243860?label=Discord)](https://discord.gg/xqNA5jt6DN)
![Количество загрузок](https://img.shields.io/github/downloads/AeEn123/RoExtract/total?label=Количество%20загрузок)

# Дисклеймер
Это независимый и образовательный проект. RoExtract никак **НЕ** связан с Roblox Corporation.

# RoExtract
Этот инструмент извлекает кэшированные данные из установленного клиента Roblox путем анализа заголовков файлов кэша.

![Скриншот](/assets/screenshot.png)

# RoExtract? И дисклеймер? А репозиторий заблокировали? Что случилось?
Недавно репозиторий был заблокирован из-за нарушений прав на товарные знаки.

У меня получилось вернуть его, поддержка GitHub хотела дисклеймер. (спасибо поддержке GitHub, всё-таки у них хорошая команда)

Дисклеймер был добавлен на сайт и README, чтобы обеспечить соответствие требованиям.

Я также переименовал проект в RoExtract, чтобы избежать дальнейших проблем с товарными знаками, так как прошлое название содержало слово "Roblox".

Ребрендинг сломал несколько ссылок, но мы приводим всё в порядок.

Спасибо за вашу поддержку этого проекта :), казалось будто я лишился всего, когда я потерял этот репозиторий.

# ЧаВО
### Запуск программы невозможен, так как на компьютере отсутствует vcruntime140.dll. 
Установите [Microsoft Visual C++ Redistributable](https://learn.microsoft.com/ru-ru/cpp/windows/latest-supported-vc-redist)

### Влияет ли это на клиент Roblox?
Нет, этот инструмент открывает файлы, которые ваш клиент уже создал. Вы можете посмотреть их сами в %Temp%\Roblox

### Это вирус?
Нет, это не вирус. Как и у любого другого свободного ПО с открытым исходным кодом, наш код открыт для просмотра кем угодно, поэтому спрятать в нём что-то вредоносное невозможно. Сборки также безопасны, поскольку теперь они создаются автоматически через GitHub Actions - это гарантирует полную прозрачность. Если вы всё равно не доверяете этому ПО, вы можете воспользоваться веб-демо по ссылке https://aeen123.github.io/RoExtract/demo (для этого ничего скачивать не нужно)

### Windows говорит "Windows защитила ваш ПК". Что мне делать?
Это окно появляется, если Windows обнаруживает программу от неизвестного издателя. В таком случае просто нажмите "Подробнее", а затем - "Выполнить в любом случае".

### Это может вызвать бан?
Нет, в отличие от читов, это ПО **не** взаимодействует с Roblox, делая эту программу безопасным способом извлечения ресурсов.

### Мои извлечённые ресурсы не проигрываются в медиа-плеере, что делать?
Некоторые медиа-плееры могут не поддерживать формат извлечённого фвйла. В таком случае попробуйте другой проигрыватель, поддерживающий все форматы, например VLC. **Если файл действительно повреждён, пожалуйста, [сообщите об ошибке.](https://github.com/AeEn123/RoExtract/issues)**

### Почему файлы KTX находятся в другой вкладке? Не должны ли они находиться во вкладке Текстуры?
Формально да, должны, но большинство программ для просмотра изображений не поддерживают файлы KTX, поэтому лучше перенести их в другую вкладку, чтобы избежать проблем с совместимостью. Эта вкладка предназначена только для опытных пользователей.

### Почему файлы RBXM отображаются просто как "Instance" в Roblox Studio?
Roblox Studio не поддерживает кэшированные файлы RBXM. Эти файлы могут содержать данные из игр, но детально мы пока этого не изучали.

### Занимает ли это место на диске со временем?
Ваш кэш Roblox действительно занимает всё больше места со временем, но сама программа не увеличивает использование диска, если только вы не извлекаете большое количество файлов (которые можно легко удалить).


# Использование
## Вкладки
Вы видите несколько вкладок. RoExtract распределяет файлы по категориям. Вы можете отфильтровать их, кликнув на вкладку.
## Панель инструментов
Каждая кнопка в панели инструментов позволяет вам делать разные операции с директорией или ресурсом, вы также можете открыть это меню, нажав ПКМ. Вы можете выключить панель инструментов вверху экрана в настройках, опция **Включить панель инструментов** в разделе **Поведения**.
## Навигация с помощью клавиатуры и горячие клавиши
Программа разработана так, чтобы её было удобно использовать мышью, но при этом поддерживает навигацию с помощью клавиатуры для опытных пользователей. Сочетания клавиш отображаются прямо на кнопках, чтобы вы могли быстро их увидеть.

Вы можете менять вкладки с помощью Alt (или Ctrl) + 1-8. Вы можете выбирать ресурсы с помощью Tab и открывать с помощью Enter.
## Меню настроек
В меню настроек вы найдёте общие параметры кастомизации и также выбор действий с кэшем. Здесь вы можете распаковать весь кэш, сменить директорию или очистить кэш.

# Режим CLI
Режим CLI ещё в разработке.
Читайте [CLI.md](/docs/ru-RU/CLI.md)

# Установка на Windows
Сейчас программа на Windows поставляется только в портативном виде, но в будущем это может измениться.

# Установка на Linux
## Flatpak (ЭКСПЕРИМЕНТАЛЬНО)
> [!WARNING]
> Поддержка Flatpak ЭКСПЕРИМЕНТАЛЬНАЯ, используйте на свой страх и риск.

На данный момент готовых пакетов Flatpak нет. Следуйте инструкциям в [руководстве](packages/flatpak/README.md), чтобы собрать пакет самостоятельно.

## Arch Linux
Вы можете установить RoExtract в Arch Linux, используя файл `PKGBUILD`, который находится в `packages/arch`
Пример установочного скрипта:
```bash
mkdir /tmp/RoExtract
cd /tmp/RoExtract
wget raw.githubusercontent.com/AeEn123/RoExtract/refs/heads/main/packages/arch/PKGBUILD
makepkg -si
```

## Другие дистрибутивы
Other distros will hopefully be supported soon. If you know how to make one and want it merged in this project, create a pull request!
Другие дистрибутивы, надеюсь, скоро будут поддерживаться. Если вы знаете как поддерживать какой-либо и хотите помочь проекту, <...>

# Testing development builds
The development builds can be downloaded from the [releases](https://github.com/AeEn123/RoExtract/releases) page.

If you already have the latest development build of RoExtract installed, you can enable development builds in settings 
# More Info
This is my first project written in rust/egui so bugs may appear, in the circumstance that a bug does appear, report an issue.

> [!IMPORTANT]
> Этот инструмент предназначен для Windows и GNU/Linux, и может не работать на других ОС.

> [!TIP]
> If file listing is too slow, you can clear your cache with the clear cache button in the settings. Also, turning off Windows Defender will speed up file listing, as it scans every time a file is opened.

# Building from source

Building from source requires cargo, [which can be installed from rustup.](https://rustup.rs/)

## 1. Clone the repository
```bash
git clone https://github.com/AeEn123/RoExtract
cd RoExtract
```
## 2. Build with cargo, the command you run depends on your use-case
If you want a finished build which runs fast but compiles slowly (recommended for normal use)
```bash
cargo build --release
```

If you want a development build which runs slowly but compiles fast (recommended for development)
```bash
cargo build
```
Wait for it to build all the dependencies and the application. After that you should find it in the `target` folder.

# Python-версия
Python-версия больше не поддерживается.