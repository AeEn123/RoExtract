# Language info
language-name = English

# Tabs
music = Music
sounds = Sounds
images = Images
rbxm-files = RBXM Files
ktx-files = KTX Files
settings = Settings
about = About
logs = Logs

# Buttons
button-extract-type = Extract all of this type <F3>
button-refresh = Refresh <F5>
button-clear-cache = Clear cache <Del>
button-extract-all = Extract all <F3>
button-change-cache-dir = Change cache directory
button-reset-cache-dir = Reset cache directory
button-change-sql-db = Change SQL Database
button-reset-sql-db = Reset SQL Database
button-finish = Finish
button-yes = Yes
button-no = No
button-rename = Rename <F2>
button-search = Search <Ctrl+F>
button-swap = Swap assets <F4>
button-copy-logs = Copy log to clipboard
button-export-logs = Export log to file
button-copy = Copy <Ctrl+C>
button-open = Open <Return>
button-extract-file = Extract <Ctrl+E>
button-display-image-preview = Display image previews
button-disable-display-image-preview = Stop displaying image previews
input-preview-size = Preview size

# Confirmations
confirmation-generic-confirmation-title = Confirmation
confirmation-delete-confirmation-title = Deleting files
confirmation-delete-confirmation-description = Are you sure you want to delete all files in this directory?
confirmation-filter-confirmation-title = Files are still being filtered.
confirmation-filter-confirmation-description = Are you sure you want to extract all the files while the program is still filtering the files? This will result in an unfinished extraction.
confirmation-clear-cache-title = Clearing cache
confirmation-clear-cache-description = Are you sure you want to clear your cache? The files will be regenerated when the client is loaded.
confirmation-custom-directory-title = Choose a different directory
confirmation-custom-directory-description = Do you want to choose a different cache directory?
confirmation-custom-sql-title = Choose a SQL Database
confirmation-custom-sql-description = Do you want to choose a different SQL Database?
confirmation-ban-warning-title = Potential ban alert
confirmation-ban-warning-description = Editing assets in games can cause your client to behave differently leading to potential game bans! Use at your own risk. Do you understand?

# Errors
no-files = No files to list.
error-directory-detection-title = Directory detection failed!
error-directory-detection-description = Directory detection failed! Is the client installed and you ran it at least once?
error-sql-detection-title = Database detection failed!
error-sql-detection-description = Database detection failed! Is the client installed and you ran it at least once?
error-temporary-directory-title = Failed to create a temporary directory!
error-temporary-directory-description = Error: Failed to create a temporary directory! Do you have read/write access to your temp folder? If this error continues, try running as administrator
error-invalid-directory-title = Invalid directory!
error-invalid-directory-description = Please make sure the path you provided is a directory
error-invalid-database-title = Invalid database!
error-invalid-database-description = Please make sure the path you provided is an SQLite Database
generic-error-critical = Critical error

# Headings
actions = Actions
updates = Updates
language-settings = Language settings
new-updates = New updates available
contributors = Contributors
dependencies = Dependencies
behavior = Behavior

# Checkboxes
check-for-updates = Check for updates
automatically-install-updates = Automatically install updates
use-alias = Export your renamed filenames
use-topbar-buttons = Enable toolbar
refresh-before-extract = Refresh file list before extracting
download-development-build = Use development builds to get the latest features early (These builds may be unstable)
checkbox-hide-user-logs = Hide username from logs


# Descriptions
clear-cache-description = If it is taking too long to list files and extracting all from a directory, you can clear your cache with the button below. This removes all files from your cache and your client will automatically recreate these files when needed.
extract-all-description = The button below will copy all assets and create folders e.g /sounds, /images to catagorize them. You can choose the root folder when starting.
custom-cache-dir-description = If you want to access a different cache, change your cache directory below, you can set it back to default with the other button. This is different from your installation folder.
custom-sql-db-description = If you want to access a different cache, change your SQL Database below, you can set it back to default with the other button. This is different from your installation folder.
use-alias-description = Instead of exporting the raw file name for the asset, checking this checkbox will export the filename you choose instead. You can do this by renaming within the application itself.
swap-choose-file = Double click a file to swap
swap-with = Double click a file to swap with "{ $asset }"
logs-description = The logs show how the program is performing, if any errors happen, they will show up here
copy-choose-file = Double click a file to copy
overwrite-with = Double click a file to overwrite with "{ $asset }"


# Statuses
idling = Idling
deleting-files = Deleting files ({ $item }/{ $total })
reading-files = Reading files ({ $item }/{ $total })
extracting-files = Extracting files ({ $item }/{ $total })
filtering-files = Filtering files ({ $item }/{ $total })
all-extracted = All files extracted
stage = Stage { $stage }/{ $max }: { $status }
swapped = Swapped { $item_a } with { $item_b }
copied = Overwritten { $item_b } with { $item_a }

# Error Statuses
failed-deleting-file = ERROR: Failed to delete ({ $item }/{ $total })
failed-opening-file = ERROR: Failed to open file
failed-not-file = ERROR: '{ $file }' Not a file
error-extracting-file = ERROR: Failed to extract: { $error }
error-check-logs = ERROR: Check logs for more details.

# Misc
no-function = (Not functional yet)
version = Version: v{ $version } (compiled at { $date })
cache-directory = Cache directory: { $directory }
sql-database = SQL Database: { $path }
welcome = Welcome
download-update-question = Would you like to download the update?
update-changelog = Update changelog below
support-sponsor = ♥ Sponsor
support-project-donate = ♥ Donate
setting-below-restart-required = Note: Changing the setting below requires restarting the program for it to apply.