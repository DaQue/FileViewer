Gemini File Viewer (Python, PySide6)

Summary
- Desktop viewer for text and images with: Open, Recent, Fit to Window, Zoom +/-/100%, Find, basic status.

Requirements
- Python 3.10+
- pip install -r requirements.txt

Run
- python main.py

Notes
- Uses only PySide6 (Qt). No Pillow required; Qt loads common image formats.
- Settings/temp recents stored in memory for now; wire to disk later if desired.

