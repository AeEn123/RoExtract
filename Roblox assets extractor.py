ver = 0.04
print(f"Roblox assets extractor v{ver}")
print("✓ Byfron friendly")
print("Please report bugs to https://github.com/AeEn123/Roblox-assets-extractor/issues")
print("-----{Now loading}-----")
# Import modules
import os
import shutil
import tempfile
import threading

try:
    from requests import get
except ImportError:
    os.system("pip install requests")
    from requests import get
try:
    import tkinter as tk
except ImportError:
    os.system("pip install tk")
    import tkinter as tk
from tkinter import ttk, messagebox, filedialog

# Check for updates
r = get("https://raw.githubusercontent.com/AeEn123/Roblox-assets-extractor/main/version.txt", timeout=5)
newver = float(r.text.strip())

# Function to do thread's work
def delete_directory_contents_thread(directory):
    counter = 0
    try:
        files = len(os.listdir(directory))
        for filename in os.listdir(directory):
            file_path = os.path.join(directory, filename)
            if os.path.isfile(file_path):
                os.remove(file_path)
                counter += 1
                status_label.config(text=f"Deleting files: {counter}/{files}")
        messagebox.showinfo("Success", "All files deleted successfully.")
        status_label.config(text="Idling")
    except Exception as e:
        print(e)
        messagebox.showerror("Error", f"Failed on file {counter}:\n {str(e)}")

# Function to start thread
def delete_directory_contents(directory):
    status_label.config(text="Deleting files...")
    threading.Thread(target=delete_directory_contents_thread, args=(directory,)).start()

# Function to thread's work
def list_files_thread(directory):
    global listingFiles
    listingFiles = True
    file_list.delete(0, tk.END)  # Clear the current file list
    files = len(os.listdir(directory))
    counter = 0
    for file_name in os.listdir(directory):
        file_path = os.path.join(directory, file_name)
        if os.path.isfile(file_path):
            if os.path.basename(directory) == "sounds":
                file_list.insert(tk.END, file_name)
            else:
                with open(file_path, "rb") as file:
                    data = file.read(1024)
                if b"OggS" in data and current_tab_name == "Sounds":
                    file_list.insert(tk.END, file_name)
                if b"PNG" in data and current_tab_name == "Textures":
                    file_list.insert(tk.END, file_name)
                counter += 1
                status_label.config(text=f"Listing files: {counter}/{files}")
    listingFiles = False
    status_label.config(text="Idling")

# Function to start thread
def list_files(directory):
    if not listingFiles:
        status_label.config(text="Listing files...")
        threading.Thread(target=list_files_thread, args=(directory,)).start()


def on_file_double_click(event):
    status_label.config(text="Extracting file...")
    selected_file = file_list.get(file_list.curselection())
    source_path = os.path.join(current_directory.get(), selected_file)
    if current_tab_name != "Textures":
        if current_tab_name == "Sounds":
            with open(source_path, "rb") as file:
                data = file.read(1024)
            if b"OggS" in data:
                dest_path = os.path.join(temporary_directory, selected_file + '.ogg')
                with open(source_path, "rb") as f:
                    data = f.read()                    
                OGGHead = data.find(b"OggS")
                data = data[OGGHead::]
                with open(dest_path, "wb") as f:
                    f.write(data)
        else:
            dest_path = os.path.join(temporary_directory, selected_file + '.ogg')
            shutil.copy2(source_path, dest_path)
    else:
        dest_path = os.path.join(temporary_directory, selected_file + '.png')
        with open(source_path, "rb") as f:
            data = f.read()
        PNGHead = data.find(b"PNG")-1
        data = data[PNGHead::]
        with open(dest_path, "wb") as f:
            f.write(data)
    os.system("start " + dest_path)
    status_label.config(text="Idling")

def on_tab_change(event):
    global current_tab_name
    selected_tab = event.widget.select()
    selected_tab_id = event.widget.index(selected_tab)
    selected_tab_idx = list(tab_directory_map.keys())[selected_tab_id]
    
    current_directory.set(tab_directory_map[selected_tab_idx])
    current_tab_name = event.widget.tab(selected_tab, option="text")
    list_files(current_directory.get())    

def add_tab(directory, name=None):
    tab_name = os.path.basename(directory)
    if name != None:
        tab_name = name
    else:
        tab_name = os.path.basename(directory)
    tab = ttk.Frame(tab_control)
    tab_control.add(tab, text=tab_name)
    tab_control.pack(expand=1, fill=tk.BOTH)
    tab_directory_map[tab] = directory
    

def delete_all_files(event=None):
    if messagebox.askyesno("Deleting...", "Are you sure you want to delete all files?"):
        delete_directory_contents(current_directory.get())
        list_files(current_directory.get())

def refresh(event=None):
    list_files(current_directory.get())

# Function to do thread's work
def extract_all_from_directory_thread():
    destination = filedialog.askdirectory()
    if destination == "": return
    directory = current_directory.get()
    try:
        files = len(os.listdir(directory))
        counter = 0
        for filename in os.listdir(directory):
            file_path = os.path.join(directory, filename)
            if os.path.isfile(file_path):
                source_path = os.path.join(directory, os.path.basename(file_path))
                if current_tab_name != "Textures":
                    if current_tab_name == "Sounds":
                        with open(file_path, "rb") as file:
                            data = file.read(1024)
                        if b"OggS" in data:
                            dest_path = os.path.join(destination, os.path.basename(file_path) + '.ogg')
                            with open(source_path, "rb") as f:
                                data = f.read()
                            OGGHead = data.find(b"OggS")
                            data = data[OGGHead::]
                            with open(dest_path, "wb") as f:
                                f.write(data)
                    else:
                        dest_path = os.path.join(destination, os.path.basename(file_path) + '.ogg')
                        shutil.copy2(source_path, dest_path)
                else:
                    with open(file_path, "rb") as file:
                        data = file.read(1024)
                    if b"PNG" in data:
                        dest_path = os.path.join(destination, os.path.basename(file_path) + '.png')
                        with open(source_path, "rb") as f:
                            data = f.read()
                        PNGHead = data.find(b"PNG")-1
                        data = data[PNGHead::]
                        with open(dest_path, "wb") as f:
                            f.write(data)
            counter += 1
            status_label.config(text=f"Extracting files: {counter}/{files}")
        messagebox.showinfo("Success", "All files were copied successfully.")
    except Exception as e:
        print(e)
        messagebox.showerror("Error", f"Failed to copy files: {str(e)}")
    status_label.config(text="Idling")

# Function to start thead
def extract_all_from_directory(event=None):
    status_label.config(text="Extracting files...")
    threading.Thread(target=extract_all_from_directory_thread).start()

# Init variables
listingFiles = False

# Create window
root = tk.Tk()
root.title(f"Roblox assets extractor v{ver}")

# Update diolgue
if newver > ver:
    r = get("https://raw.githubusercontent.com/AeEn123/Roblox-assets-extractor/main/latestchangelog.txt", timeout=5)
    if messagebox.askyesno("Update available", r.text + "\n\nDo you want to install the new update?"):
        r = get("https://raw.githubusercontent.com/AeEn123/Roblox-assets-extractor/main/Roblox%20assets%20extractor.py")
        with open(__file__, "wb") as f:
            f.write(r.content)
        import subprocess
        subprocess.Popen(["python", __file__])
        os._exit(0)

# Create temporary directory
temporary_directory_object = tempfile.TemporaryDirectory()
temporary_directory = temporary_directory_object.name

# Create tab control
tab_frame = tk.Frame(root)
tab_frame.pack(fill=tk.BOTH, expand=False)
tab_control = ttk.Notebook(tab_frame)
tab_directory_map = {}

# Add initial tab for main directory
home_directory = os.path.expandvars("%Temp%\\Roblox\\sounds")
current_directory = tk.StringVar(value=home_directory)
add_tab(home_directory, "Music")
current_tab_name = "Music"

# Create other tabs
add_tab(os.path.expandvars("%Temp%\\Roblox\\http"), "Sounds")
add_tab(os.path.expandvars("%Temp%\\Roblox\\http"), "Textures")

# Create buttons
button_frame = tk.Frame(root)
button_frame.pack()

delete_button = tk.Button(button_frame, text="Delete All Files", command=delete_all_files)
delete_button.pack(side=tk.LEFT)

refresh_button = tk.Button(button_frame, text="Refresh <F5>", command=refresh)
refresh_button.pack(side=tk.RIGHT)
root.bind('<F5>', refresh)

extract_button = tk.Button(button_frame, text="Extract all from this directory", command=extract_all_from_directory)
extract_button.pack(side=tk.RIGHT)

# Create status
status_label = tk.Label(root, text="Idling", bd=1, relief=tk.SUNKEN, anchor=tk.W)
status_label.pack(side=tk.BOTTOM, fill=tk.X)

# Create file list
file_list = tk.Listbox(root, width=50)
file_list.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)

# Create a Scrollbar widget
scrollbar = ttk.Scrollbar(root, orient=tk.VERTICAL, command=file_list.yview)
scrollbar.pack(side=tk.RIGHT, fill=tk.Y)

# Configure the Listbox to use the Scrollbar
file_list.config(yscrollcommand=scrollbar.set)

# Set up double-click event on file list
file_list.bind("<Double-Button-1>", on_file_double_click)

# Set up tab change event
tab_control.bind("<<NotebookTabChanged>>", on_tab_change)

# Start GUI event loop
print("All modules loaded")
root.mainloop()
temporary_directory_object.cleanup()
