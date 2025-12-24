### Создание Flatpak-пакета RoExtract
# Сборка и установка
1. Клонируйте репозиторий
```bash
git clone https://github.com/AeEn123/RoExtract
```
2. Перейдите в директорию `packages/flatpak` в клонированном репозитории
```bash
cd RoExtract/packages/flatpak
```
3. Выполните команду ниже, чтобы собрать Flatpak-пакет и установить его в систему
```bash
flatpak-builder --force-clean --user --install-deps-from=flathub --repo=repo --install builddir io.github.AeEn123.RoExtract.yml
```
# Создание .flatpak файла
1. Выполните действия из раздела "Сборка и установка" и продолжайте в той же папке
2. Выполните команду ниже, чтобы собрать .flatpak файл
```bash
flatpak build-bundle repo RoExtract-linux.flatpak io.github.AeEn123.RoExtract --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo
```
