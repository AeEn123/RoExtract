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
button-extract-type = Распаковать всё из этого типа <F3>
button-refresh = Перезагрузить <F5>
button-clear-cache = Очистить кэш <Del>
button-extract-all = Распаковать всё <F3>
button-change-cache-dir = Изменить директории кэша
button-reset-cache-dir = Сбросить директорию кэша
button-change-sql-db = Изменить базу данных SQL
button-reset-sql-db = Сбросить базу данных SQL
button-finish = Закончить
button-yes = Да
button-no = Нет
button-rename = Переименовать <F2>
button-search = Поиск <Ctrl+F>
button-swap = Поменять ассеты <F4>
button-copy-logs = Копировать журнал в буфер обмена
button-export-logs = Экспорт логов в файл
button-copy = Копировать <Ctrl+D>
button-open = Открыть <Return>
button-extract-file = Извлечь <Ctrl+E>
button-display-image-preview = Показать предварительный осмотр изображения
button-disable-display-image-preview = Прекратить осмотр изображения
input-preview-size = Предварительный осмотр изображения

# Confirmations
confirmation-custom-sql-title = Choose a SQL Database # TODO: Translate
confirmation-custom-sql-description = Do you want to choose a different SQL Database? # TODO: Translate
confirmation-generic-confirmation-title = Подтверждение
confirmation-delete-confirmation-title = Удаление файлов
confirmation-delete-confirmation-description = Вы уверены, что хотите удалить все файлы в этом каталоге?
confirmation-filter-confirmation-title = Файлы все еще фильтруются.
confirmation-filter-confirmation-description = Вы уверены, что хотите извлечь все файлы, пока программа ещё фильтрует их? Это приведёт к тому, что извлечение не будет завершено.
confirmation-clear-cache-title = Очистка кэша
confirmation-clear-cache-description = Вы уверены, что хотите очистить кэш? Файлы будут восстановлены при загрузке клиента.
confirmation-custom-directory-title = Выбрать другую директория
confirmation-custom-directory-description = Хотите выбрать другой директорию кэша?
confirmation-ban-warning-title = Предупреждение о возможном запрете
confirmation-ban-warning-description = Редактирование ресурсов в играх может привести к изменению поведения вашего клиента, что может привести к блокировке игры! Используйте на свой страх и риск. Вы понимаете?

# Errors
no-files = Нет файлов в списке. 
error-directory-detection-title = Не удалось обнаружить директорию!
error-directory-detection-description = Не удалось обнаружить директорию! Клиент установлен и вы его запускали хотя бы один раз?
error-sql-detection-title = Ошибка обнаружения базы данных!
error-sql-detection-description = База данных не обнаружена! Клиент установлен и вы его запускали хотя бы один раз?
error-temporary-directory-title = Не удалось создать временную директорию!
error-temporary-directory-description = Ошибка: Не удалось создать временную директорию! Есть ли у вас права на чтение или запись во временную папку? Если ошибка повторится, попробуйте запустить от имени администратора.
error-invalid-directory-title = Неверный директория!
error-invalid-directory-description = Пожалуйста, убедитесь, что указанный вами путь является каталогом.
error-invalid-database-title = Неверная база данных!
error-invalid-database-description = Убедитесь, что указанный вами путь соответствует базе данных SQLite.
generic-error-critical = Критическая ошибка

# Headings
actions = Действия
updates = Обновления
language-settings = Настройки языка
new-updates = Доступно новые обновления
contributors = Авторы
dependencies = Зависимости
behavior = Поведения

# Checkboxes
check-for-updates = Проверить наличие обновлений
automatically-install-updates = Автоматически устанавливать обновления
use-alias = Экспортировать ваши переименнованные файлы
use-topbar-buttons = Включить панель инструментов
refresh-before-extract = Обновите список файлов перед извлечением
download-development-build = Используйте сборку для разработчиков, чтобы получать новейшие функции заранее (эти сборки могут быть нестабильными)
checkbox-hide-user-logs = Скрыть имя пользователя из журналов


# Descriptions
clear-cache-description = Если вывод списка файлов и извлечение всех файлов из каталога занимает слишком много времени, вы можете очистить кэш с помощью кнопки ниже. Это удалит все файлы из кэша, и ваш клиент автоматически создаст их заново при необходимости.
extract-all-description = Кнопка ниже скопирует все ресурсы и создаст папки, например, /sounds, /images, для их классификации. Вы можете выбрать корневую папку при запуске.
custom-cache-dir-description = Если вы хотите получить доступ к другому кэшу, измените директорию кэша ниже. Вы можете вернуть его в директорию по умолчанию с помощью другой кнопки. Это отличается от папки установки.
custom-sql-db-description = Если вы хотите получить доступ к другому кэшу, измените базу данных SQL ниже. Вы можете вернуть его в директорию по умолчанию с помощью другой кнопки. Это отличается от папки установки.
use-alias-description = Вместо экспорта исходного имени файла ресурса, установка этого флажка приведет к экспорту выбранного вами имени файла. Вы можете сделать это, переименовав файл в самом приложении.
swap-choose-file = Нажмите дважды на файл для замены
swap-with = Нажмите дважды на файл для замены на "{ $asset }"
logs-description = Логи показывают, как работает программа. Если возникнут какие-либо ошибки, они будут отображены здесь.
copy-choose-file = Нажмите дважды на файл чтобы скопировать
overwrite-with = Нажмите дважды на файл чтобы перезаписать его на "{ $asset }"


# Statuses
idling = Idling
deleting-files = Удаление файлов ({ $item }/{ $total })
reading-files = Прочтение файлов ({ $item }/{ $total })
extracting-files = Извлечение файлов ({ $item }/{ $total })
filtering-files = Фильтрация файлов ({ $item }/{ $total })
all-extracted = Все файлы извлечены
stage = Стадия { $stage }/{ $max }: { $status }
swapped = { $item_a } сменился на { $item_b }
copied = { $item_b } перезаписан на { $item_a }

# Error Statuses
failed-deleting-file = ОШИБКА: Не удалось удалить ({ $item }/{ $total })
failed-opening-file = ОШИБКА: Не удалось открыть файл
failed-not-file = ОШИБКА: '{ $file }' Не файл
error-extracting-file = ОШИБКА: Не удалось извлечь: { $error }
error-check-logs = ОШИБКА: Более подробную информацию смотрите в логах.

# Misc
no-function = (Пока не функционирует)
version = Версия: v{ $version } (скомпилировано в { $date })
cache-directory = Директория кэша: { $directory }
sql-database = База данных SQL: { $path }
welcome = Добро пожаловать
download-update-question = Хотите загрузить обновление?
update-changelog = Списков изменений посмотреть ниже
support-sponsor = ♥ Спонсорство
support-project-donate = ♥ Поддержать
setting-below-restart-required = Примечание: для применения изменений приведенных ниже настроек потребуется перезапустить программу.