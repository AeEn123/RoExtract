# Language info
language-name = Русский

# Tabs
music = Музыка
sounds = Звуки
images = Изображения
rbxm-files = Файлы RBXM
ktx-files = Файлы KTX
settings = Настройки
about = О нас
logs = Журналы

# Buttons
button-extract-type = Распаковать все файлы этого типа <F3>
button-refresh = Перезагрузить <F5>
button-clear-cache = Очистить кэш <Del>
button-extract-all = Распаковать всё <F3>
button-change-cache-dir = Сменить директорию кэша
button-reset-cache-dir = Сбросить директорию кэша
button-change-sql-db = Изменить базу данных SQL
button-reset-sql-db = Сбросить базу данных SQL
button-finish = Закончить
button-yes = Да
button-no = Нет
button-rename = Переименовать <F2>
button-search = Поиск <Ctrl+F>
button-swap = Заменить ресурсы <F4>
button-copy-logs = Копировать журнал в буфер обмена
button-export-logs = Экспорт журнала в файл
button-copy = Копировать <Ctrl+C>
button-open = Открыть <Return>
button-extract-file = Извлечь <Ctrl+E>
button-display-image-preview = Показать предпросмотр изображений
button-disable-display-image-preview = Не показывать предпросмотр изображений
input-preview-size = Размер предпросмотра

# Confirmations
confirmation-custom-sql-title = Выбор базы данных SQL
confirmation-custom-sql-description = Вы хотите выбрать другую базу данных SQL?
confirmation-generic-confirmation-title = Подтверждение
confirmation-delete-confirmation-title = Удаление файлов
confirmation-delete-confirmation-description = Вы уверены, что хотите удалить все файлы в этой директории?
confirmation-filter-confirmation-title = Файлы всё ещё фильтруются
confirmation-filter-confirmation-description = Вы уверены, что хотите извлечь все файлы во время фильтрации? Список файлов будет неполным.
confirmation-clear-cache-title = Очистка кэша
confirmation-clear-cache-description = Вы уверены, что хотите очистить кэш? Файлы будут восстановлены при загрузке клиента.
confirmation-custom-directory-title = Выбрать другую директорию
confirmation-custom-directory-description = Вы хотите выбрать другую директорию кэша?
confirmation-ban-warning-title = Предупреждение о возможном бане
confirmation-ban-warning-description = Редактирование ресурсов в играх может привести к изменению поведения вашего клиента, что может привести к блокировке аккаунта! Используйте на свой страх и риск. Вы согласны?

# Errors
no-files = Нет файлов в списке. 
error-directory-detection-title = Не удалось обнаружить директорию!
error-directory-detection-description = Не удалось обнаружить директорию! Установлен ли клиент и запускали ли вы его хотя бы раз?
error-sql-detection-title = Ошибка обнаружения базы данных!
error-sql-detection-description = База данных не обнаружена! Установлен ли клиент и запускали ли вы его хотя бы раз?
error-temporary-directory-title = Не удалось создать временную директорию!
error-temporary-directory-description = Ошибка: Не удалось создать временную директорию! Есть ли у вас права на чтение или запись во временную папку? Если ошибка повторится, попробуйте запустить от имени администратора.
error-invalid-directory-title = Неверная директория!
error-invalid-directory-description = Пожалуйста, убедитесь, что указанный вами путь является директорией.
error-invalid-database-title = Неверная база данных!
error-invalid-database-description = Убедитесь, что указанный вами путь соответствует базе данных SQLite.
generic-error-critical = Критическая ошибка

# Headings
actions = Действия
updates = Обновления
language-settings = Настройки языка
new-updates = Доступны новые обновления
contributors = Авторы
dependencies = Зависимости
behavior = Поведение

# Checkboxes
check-for-updates = Проверить наличие обновлений
automatically-install-updates = Автоматически устанавливать обновления
use-alias = Экспортировать переименованные файлы
use-topbar-buttons = Включить панель инструментов
refresh-before-extract = Обновлять список файлов перед извлечением
download-development-build = Использовать сборку для разработчиков, чтобы получать новейшие функции заранее (эти сборки могут быть нестабильными)
checkbox-hide-user-logs = Скрыть имя пользователя из журналов


# Descriptions
clear-cache-description = Если вывод списка файлов и извлечение всех файлов из директории занимает слишком много времени, вы можете очистить кэш с помощью кнопки ниже. Это удалит все файлы из кэша, и ваш клиент автоматически создаст их заново при необходимости.
extract-all-description = Кнопка ниже скопирует все ресурсы и создаст папки, например, /sounds, /images, для их классификации. Вы можете выбрать корневую папку при запуске.
custom-cache-dir-description = Если вы хотите получить доступ к другому кэшу, измените директорию кэша ниже. Вы можете вернуть его в директорию по умолчанию с помощью другой кнопки. Это отличается от папки установки.
custom-sql-db-description = Если вы хотите получить доступ к другому кэшу, измените базу данных SQL ниже. Вы можете вернуть его в директорию по умолчанию с помощью другой кнопки. Это отличается от папки установки.
use-alias-description = Вместо экспорта исходного имени файла ресурса, установка этого флажка приведет к экспорту выбранного вами имени файла. Вы можете сделать это, переименовав файл в самом приложении.
swap-choose-file = Нажмите дважды на файл для замены
swap-with = Нажмите дважды на файл для замены на "{ $asset }"
logs-description = Журнал показывает, как работает программа. Если возникнут ошибки, они отобразятся здесь.
copy-choose-file = Нажмите дважды на файл чтобы скопировать
overwrite-with = Нажмите дважды на файл чтобы заменить им "{ $asset }"


# Statuses
idling = Простаивает
deleting-files = Удаление файлов ({ $item }/{ $total })
reading-files = Чтение файлов ({ $item }/{ $total })
extracting-files = Извлечение файлов ({ $item }/{ $total })
filtering-files = Фильтрация файлов ({ $item }/{ $total })
all-extracted = Все файлы извлечены
stage = Стадия { $stage }/{ $max }: { $status }
swapped = { $item_a } заменён на { $item_b }
copied = { $item_b } перезаписан файлом { $item_a }

# Error Statuses
failed-deleting-file = ОШИБКА: Не удалось удалить ({ $item }/{ $total })
failed-opening-file = ОШИБКА: Не удалось открыть файл
failed-not-file = ОШИБКА: '{ $file }' Не файл
error-extracting-file = ОШИБКА: Не удалось извлечь: { $error }
error-check-logs = ОШИБКА: Более подробную информацию смотрите в журнале.

# Misc
no-function = (Пока не функционирует)
version = Версия: v{ $version } (скомпилировано в { $date })
cache-directory = Директория кэша: { $directory }
sql-database = База данных SQL: { $path }
welcome = Добро пожаловать!
download-update-question = Хотите загрузить обновление?
update-changelog = Список изменений смотрите ниже
support-sponsor = ♥ Спонсорство
support-project-donate = ♥ Поддержать
setting-below-restart-required = Примечание: для применения изменений настроек ниже потребуется перезапустить программу.