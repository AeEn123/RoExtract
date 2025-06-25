# Language info
language-name = Polski

# Tabs
logs = Dzienniki
music = Muzyka
sounds = Dźwięki
images = Obrazy
rbxm-files = Pliki RBXM
ktx-files = Pliki KTX
settings = Ustawienia
about = Informacje

# Buttons
button-reset-sql-db = Reset SQL Database # TODO: Translate
button-change-sql-db = Change SQL Database # TODO: Translate
button-disable-display-image-preview = Przestań wyświetlać podgląd obrazu
button-display-image-preview = Wyświetl podgląd obrazu
input-preview-size = Rozmiar podglądu
button-open = Otwórz <Return>
button-extract-file = Wyodrębnij <Ctrl+E>
button-copy = Kopiuj <Ctrl+D>
button-copy-logs = Skopiuj dziennik do schowka
button-export-logs = Eksportuj dziennik do pliku
button-swap = Zamień zasoby <F4>
button-search = Szukaj <CTRL+F>
button-delete-this-dir = Usuń ten katalog <Del>
button-extract-type = Wyodrębnij wszystkie tego typu <F3>
button-refresh = Odśwież <F5>
button-clear-cache = Wyczyść pamięć podręczną <Del>
button-extract-all = Wyodrębnij wszystko <F3>
button-change-cache-dir = Zmień katalog pamięci podręcznej
button-reset-cache-dir = Zresetuj katalog pamięci podręcznej
button-finish = Zakończ
button-yes = Tak
button-no = Nie
button-rename = Zmień nazwę <F2>

# Confirmations
confirmation-ban-warning-description = Edytowanie zasobów w grach może spowodować inne zachowanie gry i możliwy jest ban twojego konta! Używaj pod własnym ryzykiem
confirmation-ban-warning-title = Możliwość bana
confirmation-generic-confirmation-title = Potwierdzenie
confirmation-delete-confirmation-title = Usuwanie plików
confirmation-delete-confirmation-description = Czy na pewno chcesz usunąć wszystkie pliki w tym katalogu?
confirmation-filter-confirmation-title = Pliki nadal są filtrowane…
confirmation-filter-confirmation-description = Czy jesteś pewny(a) że chcesz wyodrębnić wszystkie pliki kiedy program filtruje je? To spowoduje niedokończone wyodrębnianie.
confirmation-clear-cache-title = Czyszczenie pamięci podręcznej gry Roblox
confirmation-clear-cache-description = Czy jesteś pewny(a) że chcesz wyczyścić pamięć podręczną gry Roblox? Pliki zostaną wygenerowane ponownie po następnym otwarciu gry Roblox.
confirmation-custom-directory-title = Wybierz inny katalog
confirmation-custom-directory-description = Czy chcesz wybrać inny katalog pamięci podręcznej?

# Errors
error-invalid-database-description = Please make sure the path you provided is an SQLite Database # TODO: Translate
error-invalid-database-title = Invalid database! # TODO: Translate
error-sql-detection-title = Database detection failed! # TODO: Translate
error-sql-detection-description = Database detection failed! Is the client installed and you ran it at least once? # TODO: Translate
generic-error-critical = Błąd krytyczny
no-files = Brak plików do wyświetlenia.
error-directory-detection-title = Nie udało się wykryć katalogu
error-directory-detection-description = Wykrycie katalogu nieudane! Czy gra jest zainstalowana i czy uruchomiłeś(aś) ją raz?
error-temporary-directory-title = Nie udało się utworzyć katalogu tymczasowego.
error-temporary-directory-description = Błąd: Nie udało się stworzyć katalogu tymczasowego! Czy masz pozwolenia do odczytu/zapisu plików? Jeżeli ten błąd pojawia się ponownie spróbuj uruchomić program jako administrator.
error-invalid-directory-title = Nieprawidłowy katalog
error-invalid-directory-description = Upewnij się, że ścieżka którą podałeś(aś) jest katalogiem

# Headings
actions = Akcje
updates = Aktualizacje
language-settings = Ustawienia języka
new-updates = Nowa aktualizacja dostępna
contributors = Współtwórcy
dependencies = Zależności
behavior = Zachowanie

# Checkboxes
checkbox-hide-user-logs = Hide username from logs # TODO: Translate
download-development-build = Use development builds to get the latest features early (These builds may be unstable) # TODO: Translate
refresh-before-extract = Odśwież listę plików przed wyodrębnieniem
use-topbar-buttons = Włącz pasek narzędzi
check-for-updates = Poszukaj aktualizacji
automatically-install-updates = Automatycznie instaluj aktualizacje
use-alias = Eksportuj alternatywne nazwy plików


# Descriptions
custom-sql-db-description = If you want to access a different cache, change your SQL Database below, you can set it back to default with the other button. This is different from your installation folder. # TODO: Translate
overwrite-with = Podwójnie kliknij plik aby nadpisać z "{ $asset }"
copy-choose-file = Podwójnie kliknij plik do skopiowania.
logs-description = Dziennik pokazuję wydajność programu. Jeżeli występują jakieś błędy, tutaj one się pojawią.
swap-with = Podwójnie naciśnij plik aby zamienić zasoby z "{ $asset }"
swap-choose-file = Podwójnie naciśnij plik do zamienienia
clear-cache-description = Jeżeli wyświetlanie lub wyodrębnianie plików zajmuje za długo, możesz wyczyścić pamięć podręczną gry Roblox. Usunie to wszystkie pliki z pamięci podręcznej i gra wygeneruje pliki, kiedy są wymagane.
extract-all-description = Ten przycisk skopiuje wszystkie zasoby i stworzy katalogi np. /sounds, /images aby je kategoryzować. Możesz zmienić folder do wypakowania przed startem.
custom-cache-dir-description = Jeżeli potrzebujesz dostęp do pamięci potrzebnej innej instalacji gry Roblox, możesz zmienić katalog pamięci podręcznej poniżej, a możesz ustawić go z powrotem do domyślnego ustawienia klikając inny przycisk.
use-alias-description = Zamiast eksportowania zwykłej nazwy dla zasobu, zaznaczenie tej opcji wyeksportuje zasób z inną nazwą pliku. Możesz zrobić to w tej aplikacji.

# Statuses
swapped = Zamieniono { $item_a } z { $item_b }
copied = Nadpisano { $item_b } z { $item_a }
idling = Czuwanie
deleting-files = Usuwanie plików ({ $item }/{ $total })
reading-files = Czytanie plików ({ $item }/{ $total })
extracting-files = Wyodrębnianie plików ({ $item }/{ $total })
filtering-files = Filtrowanie plików ({ $item }/{ $total })
all-extracted = Wszystkie pliki zostały wyodrębnione.
stage = Etap { $stage }/{ $max }: { $status }

# Error Statuses
failed-deleting-file = BŁĄD: Nie udało się usunąć ({ $item }/{ $total })
failed-opening-file = BŁĄD: Nie udało się otworzyć pliku: { $error }
failed-not-file = BŁĄD: '{ $file }' Nie jest plikiem.
error-extracting-file = BŁĄD: Nie udało się wyodrębnić plików: { $error }
error-check-logs = BŁĄD: Sprawdź dziennik po więcej informacji.

# Misc
sql-database = SQL Database: { $path } # TODO: Translate
setting-below-restart-required = Note: Changing the setting below requires restarting the program for it to apply. # TODO: Translate
support-project-donate = ♥ Wesprzyj projekt darowizną
support-sponsor = ♥ Sponsoruj
no-function = (Nie działa)
version = Wersja: { $version }
cache-directory = Katalog pamięci podręcznej: { $directory }
welcome = Witamy
download-update-question = Czy chcesz pobrać aktualizację?
update-changelog = Lista zmian poniżej: