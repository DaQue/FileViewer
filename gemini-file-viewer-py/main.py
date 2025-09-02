import os
import sys
from pathlib import Path

from PySide6 import QtCore, QtGui, QtWidgets

IMG_EXTS = {"png", "jpg", "jpeg", "gif", "bmp", "webp"}
TXT_EXTS = {"txt", "rs", "py", "toml", "md", "json", "js", "html", "css"}
MAX_IMAGE_TEXTURE_BYTES = 128 * 1024 * 1024  # ~128 MB RGBA


class LineNumberArea(QtWidgets.QWidget):
    def __init__(self, editor):
        super().__init__(editor)
        self.codeEditor = editor

    def sizeHint(self):
        return QtCore.QSize(self.codeEditor.lineNumberAreaWidth(), 0)

    def paintEvent(self, event):
        self.codeEditor.lineNumberAreaPaintEvent(event)


class NumberedPlainTextEdit(QtWidgets.QPlainTextEdit):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.lineNumberArea = LineNumberArea(self)
        self.blockCountChanged.connect(self.updateLineNumberAreaWidth)
        self.updateRequest.connect(self.updateLineNumberArea)
        self.cursorPositionChanged.connect(self.highlightCurrentLine)
        self._line_numbers_enabled = True

        self.updateLineNumberAreaWidth(0)
        self.highlightCurrentLine()

    def set_line_numbers_enabled(self, enabled):
        self._line_numbers_enabled = enabled
        self.updateLineNumberAreaWidth(0)

    def lineNumberAreaWidth(self):
        if not self._line_numbers_enabled:
            return 0
        digits = 1
        max_num = max(1, self.blockCount())
        while max_num >= 10:
            max_num //= 10
            digits += 1
        space = 3 + self.fontMetrics().horizontalAdvance('9') * digits
        return space

    def updateLineNumberAreaWidth(self, _):
        self.setViewportMargins(self.lineNumberAreaWidth(), 0, 0, 0)

    def updateLineNumberArea(self, rect, dy):
        if dy:
            self.lineNumberArea.scroll(0, dy)
        else:
            self.lineNumberArea.update(0, rect.y(), self.lineNumberArea.width(), rect.height())

        if rect.contains(self.viewport().rect()):
            self.updateLineNumberAreaWidth(0)

    def resizeEvent(self, event):
        super().resizeEvent(event)
        cr = self.contentsRect()
        self.lineNumberArea.setGeometry(QtCore.QRect(cr.left(), cr.top(), self.lineNumberAreaWidth(), cr.height()))

    def lineNumberAreaPaintEvent(self, event):
        if not self._line_numbers_enabled:
            return
        painter = QtGui.QPainter(self.lineNumberArea)
        painter.fillRect(event.rect(), QtGui.QColor(240, 240, 240))

        block = self.firstVisibleBlock()
        blockNumber = block.blockNumber()
        top = self.blockBoundingGeometry(block).translated(self.contentOffset()).top()
        bottom = top + self.blockBoundingRect(block).height()

        while block.isValid() and top <= event.rect().bottom():
            if block.isVisible() and bottom >= event.rect().top():
                number = str(blockNumber + 1)
                painter.setPen(QtCore.Qt.black)
                painter.drawText(0, int(top), self.lineNumberArea.width(), self.fontMetrics().height(),
                                 QtCore.Qt.AlignRight, number)

            block = block.next()
            top = bottom
            bottom = top + self.blockBoundingRect(block).height()
            blockNumber += 1

    def highlightCurrentLine(self):
        extraSelections = []
        if not self.isReadOnly():
            selection = QtWidgets.QTextEdit.ExtraSelection()
            lineColor = QtGui.QColor(QtCore.Qt.yellow).lighter(160)
            selection.format.setBackground(lineColor)
            selection.format.setProperty(QtGui.QTextFormat.FullWidthSelection, True)
            selection.cursor = self.textCursor()
            selection.cursor.clearSelection()
            extraSelections.append(selection)
        self.setExtraSelections(extraSelections)


class ImageView(QtWidgets.QLabel):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setAlignment(QtCore.Qt.AlignCenter)
        self._pix: QtGui.QPixmap | None = None
        self.zoom = 1.0
        self.fit_to_window = False

    def set_image(self, pix: QtGui.QPixmap):
        self._pix = pix
        self.zoom = 1.0
        self.update()

    def sizeHint(self):
        if self._pix is not None:
            return QtCore.QSize(self._pix.width(), self._pix.height())
        return super().sizeHint()

    def paintEvent(self, e: QtGui.QPaintEvent) -> None:
        super().paintEvent(e)
        if not self._pix:
            return
        painter = QtGui.QPainter(self)
        pix = self._pix
        avail = self.contentsRect().size()
        if self.fit_to_window and pix.width() > 0 and pix.height() > 0:
            sx = avail.width() / pix.width()
            sy = avail.height() / pix.height()
            scale = max(0.1, min(6.0, min(sx, sy)))
        else:
            scale = self.zoom
        w = int(pix.width() * scale)
        h = int(pix.height() * scale)
        x = (self.width() - w) // 2
        y = (self.height() - h) // 2
        painter.drawPixmap(QtCore.QRect(x, y, w, h), pix)


class MainWindow(QtWidgets.QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Gemini File Viewer (Py)")
        self.resize(1000, 700)

        self._current_path: Path | None = None
        self._recents: list[Path] = []
        self._settings_path = Path.home() / ".gemini_file_viewer_py" / "settings.json"
        self._dark_mode: bool = True
        self._wrap: bool = True
        self._default_text_zoom: float = 1.0
        self._default_image_zoom: float = 1.0
        self._ensure_settings_dir()

        self.scroll = QtWidgets.QScrollArea()
        self.scroll.setWidgetResizable(True)
        self.setCentralWidget(self.scroll)

        self.text = NumberedPlainTextEdit()
        self.text.setReadOnly(True)
        self._base_text_pt = self.text.font().pointSizeF() or 10.0
        self.text_zoom = 1.0
        self.highlighter = SimpleHighlighter(self.text.document())

        self.image = ImageView()

        self.container = QtWidgets.QWidget()
        self.vbox = QtWidgets.QVBoxLayout(self.container)
        self.vbox.setContentsMargins(0, 0, 0, 0)
        self.vbox.addWidget(self.text)
        self.vbox.addWidget(self.image)
        self.scroll.setWidget(self.container)

        self.image.hide()

        self.find_bar = QtWidgets.QToolBar("Find")
        self.addToolBar(QtCore.Qt.TopToolBarArea, self.find_bar)
        self.find_edit = QtWidgets.QLineEdit()
        self.find_edit.setPlaceholderText("Find… (Ctrl+F)")
        self.find_edit.returnPressed.connect(self.find_next)
        self.find_bar.addWidget(self.find_edit)
        self.find_count = QtWidgets.QLabel("")
        self.find_bar.addWidget(self.find_count)

        tb = QtWidgets.QToolBar("Main")
        self.addToolBar(QtCore.Qt.TopToolBarArea, tb)
        act_open = tb.addAction("Open…")
        act_open.triggered.connect(self.open_dialog)
        self.recent_menu = QtWidgets.QMenu("Recent")
        btn_recent = QtWidgets.QToolButton()
        btn_recent.setText("Recent")
        btn_recent.setMenu(self.recent_menu)
        btn_recent.setPopupMode(QtWidgets.QToolButton.InstantPopup)
        tb.addWidget(btn_recent)
        tb.addSeparator()
        self.chk_dark = QtWidgets.QCheckBox("Dark Mode")
        self.chk_dark.setChecked(self._dark_mode)
        self.chk_dark.stateChanged.connect(self.toggle_dark)
        tb.addWidget(self.chk_dark)
        self.apply_theme(self._dark_mode)
        tb.addSeparator()
        self.chk_wrap = QtWidgets.QCheckBox("Word Wrap")
        self.chk_wrap.setChecked(self._wrap)
        self.chk_wrap.stateChanged.connect(self.toggle_wrap)
        tb.addWidget(self.chk_wrap)
        # apply initial wrap
        self.apply_wrap(self._wrap)
        tb.addSeparator()
        self.chk_ln = QtWidgets.QCheckBox("Line Numbers")
        self.chk_ln.setChecked(True)
        self.chk_ln.stateChanged.connect(self.toggle_line_numbers)
        tb.addWidget(self.chk_ln)
        tb.addSeparator()
        self.text.set_line_numbers_enabled(self.chk_ln.isChecked())
        self.chk_fit = QtWidgets.QCheckBox("Fit to Window")
        self.chk_fit.stateChanged.connect(self.toggle_fit)
        tb.addWidget(self.chk_fit)
        act_zm_out = tb.addAction("Zoom -")
        act_zm_out.triggered.connect(lambda: self.zoom_image(1/1.1))
        act_zm_in = tb.addAction("Zoom +")
        act_zm_in.triggered.connect(lambda: self.zoom_image(1.1))
        act_zm_100 = tb.addAction("100%")
        act_zm_100.triggered.connect(self.reset_zoom)
        tb.addSeparator()
        act_clear = tb.addAction("Clear")
        act_clear.triggered.connect(self.clear)
        tb.addSeparator()
        self.act_copy = tb.addAction("Copy Path")
        self.act_copy.triggered.connect(self.copy_path)
        self.act_open_folder = tb.addAction("Open Folder")
        self.act_open_folder.triggered.connect(self.open_folder)

        self.status = self.statusBar()

        # Shortcuts
        QtGui.QShortcut(QtGui.QKeySequence("Ctrl+O"), self, activated=self.open_dialog)
        QtGui.QShortcut(QtGui.QKeySequence("Ctrl+F"), self, activated=self.find_edit.setFocus)
        QtGui.QShortcut(QtGui.QKeySequence("Ctrl++"), self, activated=lambda: self.zoom_image(1.1))
        QtGui.QShortcut(QtGui.QKeySequence("Ctrl+-"), self, activated=lambda: self.zoom_image(1/1.1))
        QtGui.QShortcut(QtGui.QKeySequence("Ctrl+0"), self, activated=self.reset_zoom)

        self.text.installEventFilter(self)
        self.image.installEventFilter(self)
        
        self._load_settings()
        
        # apply initial text zoom from settings
        try:
            self.apply_text_zoom(self.text_zoom)
        except Exception:
            pass

    def eventFilter(self, obj, ev):
        if obj is self.image and ev.type() == QtCore.QEvent.Wheel:
            delta = ev.angleDelta().y()
            if delta:
                self.image.fit_to_window = False
                self.image.zoom = max(0.1, min(6.0, self.image.zoom * (1.1 if delta > 0 else 1/1.1)))
                self.image.update()
                self._update_image_status()
                return True
        if obj is self.text and ev.type() == QtCore.QEvent.Wheel and (QtWidgets.QApplication.keyboardModifiers() & QtCore.Qt.ControlModifier):
            delta = ev.angleDelta().y()
            if delta:
                self.text_zoom = max(0.6, min(3.0, self.text_zoom * (1.05 if delta > 0 else 1/1.05)))
                self.apply_text_zoom(self.text_zoom)
                self._save_settings()
                return True
        return super().eventFilter(obj, ev)

    def open_dialog(self):
        dlg = QtWidgets.QFileDialog(self, "Open File")
        dlg.setFileMode(QtWidgets.QFileDialog.ExistingFile)
        filters = [
            "All Supported (*.txt *.rs *.py *.toml *.md *.json *.js *.html *.css *.png *.jpg *.jpeg *.gif *.bmp *.webp)",
            "Images (*.png *.jpg *.jpeg *.gif *.bmp *.webp)",
            "Text/Source (*.txt *.rs *.py *.toml *.md *.json *.js *.html *.css)",
        ]
        dlg.setNameFilters(filters)
        if dlg.exec() == QtWidgets.QDialog.Accepted:
            self.load_path(Path(dlg.selectedFiles()[0]))

    def toggle_fit(self, state):
        self.image.fit_to_window = bool(state)
        self.image.update()
        self._update_image_status()
        self._save_settings()

    def toggle_dark(self, state):
        self._dark_mode = bool(state)
        self.apply_theme(self._dark_mode)
        self._save_settings()

    def toggle_wrap(self, state):
        self._wrap = bool(state)
        self.apply_wrap(self._wrap)
        self._save_settings()

    def apply_wrap(self, enabled: bool):
        mode = QtWidgets.QPlainTextEdit.WidgetWidth if enabled else QtWidgets.QPlainTextEdit.NoWrap
        self.text.setLineWrapMode(mode)

    def toggle_line_numbers(self, state):
        on = bool(state)
        # Downgrade if very large
        if self.text.blockCount() > 50000:
            on = False
            self.chk_ln.setChecked(False)
        self.text.set_line_numbers_enabled(on)
        self._save_settings()

    def zoom_image(self, factor: float):
        if not self.image.isVisible():
            return
        self.image.fit_to_window = False
        self.chk_fit.setChecked(False)
        self.image.zoom = max(0.1, min(6.0, self.image.zoom * factor))
        self.image.update()
        self._update_image_status()

    def reset_zoom(self):
        if self.image.isVisible():
            self.image.fit_to_window = False
            self.chk_fit.setChecked(False)
            self.image.zoom = 1.0
            self.image.update()
            self._update_image_status()
        else:
            self.text_zoom = 1.0
            self.text.setFont(QtGui.QFontDatabase.systemFont(QtGui.QFontDatabase.FixedFont))

    def clear(self):
        self._current_path = None
        self.text.clear()
        self.text.hide()
        self.image.hide()
        self.status.clearMessage()
        self._update_actions_enabled()

    def load_path(self, path: Path):
        self._current_path = path
        ext = path.suffix.lower().lstrip(".")
        try:
            if ext in IMG_EXTS:
                # Pre-check dimensions without fully decoding
                reader = QtGui.QImageReader(str(path))
                size = reader.size()
                if size.width() > 0 and size.height() > 0:
                    est = size.width() * size.height() * 4
                    if est > MAX_IMAGE_TEXTURE_BYTES:
                        raise RuntimeError(
                            f"Image too large: {size.width()}x{size.height()} (~{est/1024/1024:.1f} MB RGBA). Limit ~{MAX_IMAGE_TEXTURE_BYTES/1024/1024:.0f} MB"
                        )
                pix = QtGui.QPixmap(str(path))
                if pix.isNull():
                    raise RuntimeError("Failed to load image")
                self.image.set_image(pix)
                # apply persisted image zoom if not fitting
                if not self.chk_fit.isChecked():
                    self.image.zoom = max(0.1, min(6.0, float(getattr(self, '_default_image_zoom', 1.0))))
                self.text.hide()
                self.image.show()
                self._update_image_status()
            else:
                with open(path, "rb") as f:
                    data = f.read()
                text = data.decode("utf-8", errors="replace")
                self.text.setPlainText(text)
                self.highlighter.set_language(ext)
                self.apply_wrap(self._wrap)
                # apply persisted text zoom
                try:
                    self.apply_text_zoom(float(getattr(self, '_default_text_zoom', 1.0)))
                    self.text_zoom = float(getattr(self, '_default_text_zoom', 1.0))
                except Exception:
                    pass
                self.text.show()
                self.image.hide()
                lines = text.count("\n") + 1 if text else 0
                self.status.showMessage(f"{path} — {lines} lines")
        except Exception as e:
            QtWidgets.QMessageBox.critical(self, "Error", str(e))
            return
        # recents
        if path in self._recents:
            self._recents.remove(path)
        self._recents.append(path)
        self._recents = self._recents[-10:]
        self.refresh_recents()
        self._save_settings()
        self._update_actions_enabled()

    def refresh_recents(self):
        self.recent_menu.clear()
        if not self._recents:
            act = self.recent_menu.addAction("(empty)")
            act.setEnabled(False)
        for p in reversed(self._recents):
            act = self.recent_menu.addAction(str(p))
            act.triggered.connect(lambda checked=False, pp=p: self.load_path(pp))
        self.recent_menu.addSeparator()
        clear_act = self.recent_menu.addAction("Clear Recent Files")
        def _clear():
            self._recents.clear()
            self._save_settings()
        clear_act.triggered.connect(_clear)

    def find_next(self):
        needle = self.find_edit.text()
        if not needle or not self.text.isVisible():
            return
        found = self.text.find(needle)
        if not found:
            # loop from start
            cursor = self.text.textCursor()
            cursor.movePosition(QtGui.QTextCursor.Start)
            self.text.setTextCursor(cursor)
            self.text.find(needle)
        self.update_find_count(needle)

    def update_find_count(self, needle: str):
        if not self.text.isVisible():
            self.find_count.setText("")
            return
        text = self.text.toPlainText()
        cnt = text.count(needle)
        self.find_count.setText(f"{cnt} match(es)")

    def resizeEvent(self, event: QtGui.QResizeEvent) -> None:
        super().resizeEvent(event)
        self._update_image_status()

    def _effective_image_zoom(self) -> float:
        if not self.image._pix:
            return 1.0
        if self.image.fit_to_window:
            avail = self.image.contentsRect().size()
            w = self.image._pix.width() or 1
            h = self.image._pix.height() or 1
            sx = avail.width() / w
            sy = avail.height() / h
            return max(0.1, min(6.0, min(sx, sy)))
        return self.image.zoom

    def _update_image_status(self):
        if not self.image.isVisible() or not self.image._pix:
            return
        pix = self.image._pix
        eff = self._effective_image_zoom()
        est = (pix.width() * pix.height() * 4) / (1024*1024)
        extra = " Fit: on" if self.image.fit_to_window else ""
        self.status.showMessage(f"{self._current_path} — {pix.width()}x{pix.height()} px — Zoom: {eff*100:.0f}% — Texture ~{est:.1f} MB{extra}")
        self._update_actions_enabled()

    def _update_actions_enabled(self):
        has_path = self._current_path is not None
        self.act_copy.setEnabled(has_path)
        self.act_open_folder.setEnabled(has_path)

    def copy_path(self):
        if not self._current_path:
            return
        QtWidgets.QApplication.clipboard().setText(str(self._current_path))
        self.status.showMessage("Path copied to clipboard", 1500)

    def open_folder(self):
        if not self._current_path:
            return
        p = self._current_path
        try:
            if sys.platform.startswith('win'):
                # Reveal in Explorer
                import subprocess
                subprocess.Popen(['explorer', '/select,', str(p)])
            elif sys.platform == 'darwin':
                import subprocess
                subprocess.Popen(['open', '-R', str(p)])
            else:
                # Linux: open containing dir
                import subprocess, os
                subprocess.Popen(['xdg-open', str(p.parent)])
        except Exception as e:
            QtWidgets.QMessageBox.warning(self, "Open Folder", f"Failed: {e}")

    def apply_text_zoom(self, factor: float):
        try:
            font = self.text.font()
            base = getattr(self, '_base_text_pt', font.pointSizeF() or 10.0)
            font.setPointSizeF(max(6.0, min(64.0, base * factor)))
            self.text.setFont(font)
        except Exception:
            pass

    def _ensure_settings_dir(self):
        self._settings_path.parent.mkdir(parents=True, exist_ok=True)

    def _load_settings(self):
        try:
            if self._settings_path.exists():
                import json
                obj = json.loads(self._settings_path.read_text(encoding="utf-8"))
                rec = obj.get("recents", [])
                self._recents = [Path(p) for p in rec if isinstance(p, str)]
                self._dark_mode = bool(obj.get("dark", True))
                self.chk_fit.setChecked(bool(obj.get("fit", False)))
                self._wrap = bool(obj.get("wrap", True))
                self.apply_theme(self._dark_mode)
                ln = obj.get("line_numbers")
                if ln is not None:
                    self.chk_ln.setChecked(bool(ln))
                    self.text.set_line_numbers_enabled(bool(ln))
                self._default_text_zoom = float(obj.get("text_zoom", 1.0))
                self._default_image_zoom = float(obj.get("image_zoom", 1.0))
                self.text_zoom = self._default_text_zoom
        except Exception:
            self._recents = []
        finally:
            self.refresh_recents()

    def _save_settings(self):
        try:
            import json
            data = {
                "recents": [str(p) for p in self._recents[-10:]],
                "dark": self._dark_mode,
                "fit": self.chk_fit.isChecked(),
                "wrap": self._wrap,
                "line_numbers": self.chk_ln.isChecked(),
                "text_zoom": round(self.text_zoom, 3),
                "image_zoom": round(self.image.zoom if hasattr(self.image, 'zoom') else 1.0, 3),
            }
            self._settings_path.write_text(json.dumps(data, indent=2), encoding="utf-8")
        except Exception:
            pass

    def apply_theme(self, dark: bool):
        app = QtWidgets.QApplication.instance()
        if not app:
            return
        if dark:
            app.setStyle("Fusion")
            palette = QtGui.QPalette()
            palette.setColor(QtGui.QPalette.Window, QtGui.QColor(53,53,53))
            palette.setColor(QtGui.QPalette.WindowText, QtCore.Qt.white)
            palette.setColor(QtGui.QPalette.Base, QtGui.QColor(25,25,25))
            palette.setColor(QtGui.QPalette.AlternateBase, QtGui.QColor(53,53,53))
            palette.setColor(QtGui.QPalette.ToolTipBase, QtCore.Qt.white)
            palette.setColor(QtGui.QPalette.ToolTipText, QtCore.Qt.white)
            palette.setColor(QtGui.QPalette.Text, QtCore.Qt.white)
            palette.setColor(QtGui.QPalette.Button, QtGui.QColor(53,53,53))
            palette.setColor(QtGui.QPalette.ButtonText, QtCore.Qt.white)
            palette.setColor(QtGui.QPalette.BrightText, QtCore.Qt.red)
            palette.setColor(QtGui.QPalette.Link, QtGui.QColor(42,130,218))
            palette.setColor(QtGui.QPalette.Highlight, QtGui.QColor(42,130,218))
            palette.setColor(QtGui.QPalette.HighlightedText, QtCore.Qt.black)
            app.setPalette(palette)
        else:
            app.setPalette(app.style().standardPalette())


class SimpleHighlighter(QtGui.QSyntaxHighlighter):
    def __init__(self, parent):
        super().__init__(parent)
        self.lang = ""
        self.rules_py = self._build_rules_py()
        self.rules_rs = self._build_rules_rs()

    def set_language(self, ext: str):
        self.lang = ext
        self.rehighlight()

    def highlightBlock(self, text: str) -> None:
        # Strings
        string_fmt = QtGui.QTextCharFormat()
        string_fmt.setForeground(QtGui.QColor(152, 195, 121))
        in_str = False
        start = 0
        i = 0
        while i < len(text):
            ch = text[i]
            if ch == '"':
                if not in_str:
                    in_str = True
                    start = i
                else:
                    self.setFormat(start, i - start + 1, string_fmt)
                    in_str = False
            i += 1
        # Comments
        comment_fmt = QtGui.QTextCharFormat()
        comment_fmt.setForeground(QtGui.QColor('gray'))
        if self.lang in ("py", "toml"):
            idx = text.find('#')
            if idx >= 0:
                self.setFormat(idx, len(text) - idx, comment_fmt)
        elif self.lang == "rs":
            idx = text.find('//')
            if idx >= 0:
                self.setFormat(idx, len(text) - idx, comment_fmt)
        # Keywords
        kw_fmt = QtGui.QTextCharFormat()
        kw_fmt.setForeground(QtGui.QColor(97, 175, 239))
        for rx in (self.rules_py if self.lang == "py" else self.rules_rs if self.lang == "rs" else []):
            for m in rx.finditer(text):
                self.setFormat(m.start(), m.end() - m.start(), kw_fmt)
        # Numbers
        num_fmt = QtGui.QTextCharFormat()
        num_fmt.setForeground(QtGui.QColor(209, 154, 102))
        import re as _re
        for m in _re.finditer(r"\b\d+\b", text):
            self.setFormat(m.start(), m.end() - m.start(), num_fmt)

    def _build_rules_py(self):
        import re
        kws = (
            r"False|None|True|and|as|assert|async|await|break|class|continue|def|del|elif|else|except|finally|for|from|global|if|import|in|is|lambda|nonlocal|not|or|pass|raise|return|try|while|with|yield|match|case"
        )
        return [re.compile(fr"\b({kws})\b")]

    def _build_rules_rs(self):
        import re
        kws = (
            r"as|async|await|break|const|continue|crate|dyn|else|enum|extern|false|fn|for|if|impl|in|let|loop|match|mod|move|mut|pub|ref|return|self|Self|static|struct|super|trait|true|type|unsafe|use|where|while|union|box|try|yield|macro|macro_rules"
        )
        return [re.compile(fr"\b({kws})\b")]


def main():
    app = QtWidgets.QApplication(sys.argv)
    win = MainWindow()
    win.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()