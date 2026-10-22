#!/usr/bin/env python3
"""ImgWheel Nautilus Python Extension.

Provides right-click 'Convert with ImgWheel' menu item and tracks active selection
for instant shortcut conversion (e.g. Alt+T in Hyprland).
"""

import os
import subprocess
import urllib.parse
from typing import List

try:
    import gi
    gi.require_version('Nautilus', '4.1')
    from gi.repository import Nautilus, GObject
except Exception:
    try:
        import gi
        gi.require_version('Nautilus', '4.0')
        from gi.repository import Nautilus, GObject
    except Exception:
        pass

SUPPORTED_EXTENSIONS = {
    "png", "jpg", "jpeg", "webp", "avif", "heic", "heif",
    "bmp", "eps", "gif", "tiff", "tif", "ico", "svg", "qoi", "tga",
}

class ImgWheelExtension(GObject.GObject, Nautilus.MenuProvider):
    def __init__(self):
        super().__init__()

    def get_file_items(self, files: List[Nautilus.FileInfo]) -> List[Nautilus.MenuItem]:
        image_paths = []
        for file in files:
            if file.is_directory():
                continue
            uri = file.get_uri()
            if not uri.startswith("file://"):
                continue
            # Strip file:// and unquote percent-encoding
            path = urllib.parse.unquote(uri[7:])
            ext = os.path.splitext(path)[1].lower().lstrip(".")
            mime = file.get_mime_type() if hasattr(file, 'get_mime_type') else ""
            if ext in SUPPORTED_EXTENSIONS or mime.startswith("image/"):
                image_paths.append(path)

        if not image_paths:
            return []

        # Record selection to runtime dir for instant shortcut activation
        runtime_dir = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
        selection_file = os.path.join(runtime_dir, "imgwheel_selection.txt")
        try:
            with open(selection_file, "w") as f:
                f.write("\n".join(image_paths) + "\n")
        except Exception:
            pass

        item = Nautilus.MenuItem(
            name="ImgWheel::convert",
            label="Convert with ImgWheel",
            tip="Convert image(s) using ImgWheel radial menu",
            icon="image-x-generic",
        )
        item.connect("activate", self._on_menu_activate, image_paths)
        return [item]

    def _on_menu_activate(self, menu: Nautilus.MenuItem, paths: List[str]):
        # Try daemon socket first
        runtime_dir = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
        sock_path = os.path.join(runtime_dir, "imgwheel.sock")
        if os.path.exists(sock_path):
            import socket
            import json
            try:
                s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                s.connect(sock_path)
                req = json.dumps({"files": paths, "to": None, "quality": None, "out_dir": None}) + "\n"
                s.sendall(req.encode("utf-8"))
                s.close()
                return
            except Exception:
                pass

        # Fallback: spawn imgwheel directly
        subprocess.Popen(["imgwheel"] + paths)
