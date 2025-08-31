import os
import sys
from pathlib import Path

from PySide6 import QtCore, QtGui, QtWidgets

IMG_EXTS = {"png", "jpg", "jpeg", "gif", "bmp", "webp"}
TXT_EXTS = {"txt", "rs", "py", "toml", "md", "json", "js", "html", "css"}


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

        self.scroll = QtWidgets.QScrollArea()
        self.scroll.setWidgetResizable(True)
        self.setCentralWidget(self.scroll)

        self.text = QtWidgets.QPlainTextEdit()
        self.text.setReadOnly(True)
        self.text_zoom = 1.0

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

        self.status = self.statusBar()

        # Shortcuts
        QtGui.QShortcut(QtGui.QKeySequence("Ctrl+O"), self, activated=self.open_dialog)
        QtGui.QShortcut(QtGui.QKeySequence("Ctrl+F"), self, activated=self.find_edit.setFocus)
        QtGui.QShortcut(QtGui.QKeySequence("Ctrl++"), self, activated=lambda: self.zoom_image(1.1))
        QtGui.QShortcut(QtGui.QKeySequence("Ctrl+-"), self, activated=lambda: self.zoom_image(1/1.1))
        QtGui.QShortcut(QtGui.QKeySequence("Ctrl+0"), self, activated=self.reset_zoom)

        self.text.installEventFilter(self)

    def eventFilter(self, obj, ev):
        if obj is self.image and ev.type() == QtCore.QEvent.Wheel:
            delta = ev.angleDelta().y()
            if delta:
                self.image.fit_to_window = False
                self.image.zoom = max(0.1, min(6.0, self.image.zoom * (1.1 if delta > 0 else 1/1.1)))
                self.image.update()
                return True
        if obj is self.text and ev.type() == QtCore.QEvent.Wheel and (QtWidgets.QApplication.keyboardModifiers() & QtCore.Qt.ControlModifier):
            delta = ev.angleDelta().y()
            if delta:
                self.text_zoom = max(0.6, min(3.0, self.text_zoom * (1.05 if delta > 0 else 1/1.05)))
                font = self.text.font()
                font.setPointSizeF(font.pointSizeF() * (1.05 if delta > 0 else 1/1.05))
                self.text.setFont(font)
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

    def zoom_image(self, factor: float):
        if not self.image.isVisible():
            return
        self.image.fit_to_window = False
        self.chk_fit.setChecked(False)
        self.image.zoom = max(0.1, min(6.0, self.image.zoom * factor))
        self.image.update()

    def reset_zoom(self):
        if self.image.isVisible():
            self.image.fit_to_window = False
            self.chk_fit.setChecked(False)
            self.image.zoom = 1.0
            self.image.update()
        else:
            self.text_zoom = 1.0
            self.text.setFont(QtGui.QFontDatabase.systemFont(QtGui.QFontDatabase.FixedFont))

    def clear(self):
        self._current_path = None
        self.text.clear()
        self.text.hide()
        self.image.hide()
        self.status.clearMessage()

    def load_path(self, path: Path):
        self._current_path = path
        ext = path.suffix.lower().lstrip(".")
        try:
            if ext in IMG_EXTS:
                pix = QtGui.QPixmap(str(path))
                if pix.isNull():
                    raise RuntimeError("Failed to load image")
                self.image.set_image(pix)
                self.text.hide()
                self.image.show()
                self.status.showMessage(f"{path} — {pix.width()}x{pix.height()} px")
            else:
                with open(path, "rb") as f:
                    data = f.read()
                text = data.decode("utf-8", errors="replace")
                self.text.setPlainText(text)
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
        clear_act.triggered.connect(lambda: self._recents.clear())

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


def main():
    app = QtWidgets.QApplication(sys.argv)
    win = MainWindow()
    win.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()

