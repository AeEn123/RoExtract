### Creating a Flatpak package of RoExtract
# Building and installing
1. Download the repository
```
git clone https://github.com/AeEn123/RoExtract
```
2. Change directory to the packages/flatpak folder inside the cloned repository
```
cd RoExtract/packages/folder
```
3. Run the command below to build the flatpak and install it on to your system
```
flatpak-builder --force-clean --user --install-deps-from=flathub --repo=repo --install builddir com.github.AeEn123.RoExtract.yml
```
# Creating a .flatpak file
1. Do "Building and installing" and inside the same folder continue
2. Run the command below to build the .flatpak file
```
flatpak build-bundle repo RoExtract-linux.flatpak com.github.AeEn123.RoExtract --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo
```
