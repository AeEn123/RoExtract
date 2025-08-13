# Checklist of things that need to be implemented. (ordered in priority)

## 0.1.0
- [x] - Welcome GUI (Allow user to set settings)
- [x] - Auto updates
- [x] - German locale needs updating
- [x] - Release of program (0.1.0)

## 0.1.1

- [x] - Fill in information on the blank about screen
- [x] - Release of 0.1.1

## 0.1.2
- [x] - CMD window fixed

## 0.1.3
- [x] - Naming of assets
- [x] - Add option to export named assets
- [x] - Implement all TODO comments
- [x] - Builds using GitHub Actions (Run workflow on Windows to target x86_64-pc-windows-msvc)
- [x] - Locales need updating (Deadline: 9/12/2024)

## 0.1.4
- [x] - Copy/swap assets around

## 1.0.0
- [x] - Make search case insensitive
- [x] - Allow user to toggle refresh before extract
- [x] - Move asset-specific operations into right click menu
- [x] - Implement asset copying
- [x] - image decoder to quickly preview assets
- [x] - Use versioning numbers better major.minor.patch
- [x] - Fix CLI mode
- [x] - Test if all builds work
- [x] - Fix with modern Roblox versions on Windows (probably other OSes later too!)
- [ ] - Wait for translations
- [x] - GUI for setting SQL database location


### Changelog for 1.0.0
Name: The actually major update
# Images tab
Major improvements are made to the images tab, allowing you to preview the images within the application
The images will appear in a grid-like interface just like any file manager.
# Rebrand
The project was renamed to RoExtract to avoid any trademark concerns. RoExtract is **NOT** affiliated with the Roblox Corporation in any way.
# Translations
Thanks to @MarcelDev and @Vonercent for keeping translations up-to-date.
# Minor changes
The program can now read from the SQL database that the roblox client stores assets in instead of the http folder, both are still supported.

You can now right-click on assets to see the properties, because of this, a setting has been added to remove the toolbar. The toolbar will not be removed, it is up to user-preference.

You can now copy to other assets from one asset instead of swapping them.

Version numbers now follow major.minor.patch

The option to refresh before extracting all assets can be enabled to make sure you get the latest assets extracted

The search is now case-insensitive.

Leaving the box blank for asset name edit blank will fill in with the original name as the placeholder text. This is to make it more obvious on how to remove custom names from assets.

You can now test development builds within the settings

**Full Changelog**: https://github.com/AeEn123/RoExtract/compare/v0.1.4...v1.0.0

## 1.0.1
- [x] - Update dependencies
- [x] - Fix clear cache button

# Changelog for  1.0.1
Name: Minor Update
# Security Patch
This update includes a security patch for [CVE-2025-55159](https://nvd.nist.gov/vuln/detail/CVE-2025-55159)
# Bug fix
This update includes a bug fix for the clear cache button

## 1.1.0
- [ ] - Built-in media player to quickly preview sounds
- [ ] - Audio length in list
- [ ] - Make it into a table with specific details e.g size, type, time created
- [ ] - Wait for translations

## Future releases
- [ ] - Finish CLI mode documentation
- [ ] - Community-made resource packs
- [ ] - Automatic command line generation for [Bloxstrap](https://github.com/pizzaboxer/bloxstrap)
- [ ] - Docs available in different languages