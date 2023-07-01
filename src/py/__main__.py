"""A PyQt5 visualization for Asteracer."""
import os
import sys
from typing import Optional

from PyQt5.QtCore import QTimer, Qt, QSize, QPointF, QRectF
from PyQt5.QtGui import QPainter, QBrush, QPen, QRegion, QKeySequence
from PyQt5.QtWidgets import QAction, QSpinBox, QMenuBar, QMainWindow, QApplication, QPushButton, QFrame, QLineEdit, \
    QMenu, QFileDialog

from asteracer import *


class Asteracer(QMainWindow):
    MIN_ZOOM = 0.0001
    MAX_ZOOM = 0.02
    ZOOM_MULTIPLIER = 1.2
    TICKS_PER_SECOND = 32  # TODO: this should be configurable in UI

    def save_instructions(self):
        self._stopSimulation()

        dialog = QFileDialog(self)
        filename, _ = dialog.getSaveFileName(self, "Save Instructions", filter="Text Files (*.txt);; Any File (*)")

        if not filename:
            return

        if not filename.endswith(".txt"):
            filename += ".txt"

        save_instructions(filename, self.instructions)

    def load_simulation(self, path=None):
        if self.simulation is not None:
            self._stopSimulation()

        if path:
            filename = path
        else:
            dialog = QFileDialog(self)
            filename, _ = dialog.getOpenFileName(self, "Open Simulation", filter="Text Files (*.txt);; Any File (*)")

        if not filename:
            return

        try:
            simulation = Simulation.load(filename)
        except Exception as e:
            # TODO: meaningful errors
            return

        if self.simulation is not None:
            self.tickSpinBox.setValue(0)
            self._stopSimulation()

        self.instructions = []
        self.simulation = simulation
        self.racers = [dataclasses.replace(self.simulation.racer)]
        self.reached_goals = [list(self.simulation.reached_goals)]

        self.update()

    def load_instructions(self):
        self._stopSimulation()

        dialog = QFileDialog(self)
        filename, _ = dialog.getOpenFileName(self, "Open Instructions", filter="Text Files (*.txt);; Any File (*)")

        try:
            new_instructions = load_instructions(filename)
        except:
            # TODO: meaningful errors
            return

        self.instructions = new_instructions
        self.simulation.restart()
        # TODO: is copy-pasted
        self.racers = [dataclasses.replace(self.simulation.racer)]
        self.reached_goals = [list(self.simulation.reached_goals)]

        for instruction in self.instructions:
            self.simulation.tick(instruction)
            # TODO: is copy-pasted
            self.racers.append(dataclasses.replace(self.simulation.racer))
            self.reached_goals.append(list(self.simulation.reached_goals))

        # going to the start makes more sense imo
        self.tickSpinBox.setValue(0)
        self.tick()

    def _prepareMenus(self):
        menuBar = QMenuBar(self)
        self.setMenuBar(menuBar)

        fileMenu = QMenu("&File", self)
        menuBar.addMenu(fileMenu)

        fileMenu.addAction(QAction(self, text="&Load Simulation",
                                   triggered=self.load_simulation, shortcut=QKeySequence("Ctrl+Shift+O")))
        fileMenu.addAction(QAction(self, text="&Load Instructions",
                                   triggered=self.load_instructions, shortcut=QKeySequence("Ctrl+O")))
        fileMenu.addAction(QAction(self, text="&Save Instructions",
                                   triggered=self.save_instructions, shortcut=QKeySequence("Ctrl+S")))

        aboutMenu = QMenu("&About", self)
        menuBar.addMenu(aboutMenu)

        # TODO: this should probably do something?
        aboutMenu.addAction(QAction(self, text="&About"))
        aboutMenu.addAction(QAction(self, text="&Help"))

    def _prepareTools(self):
        controlToolBar = self.addToolBar("Control")

        self.tickSpinBox = QSpinBox(focusPolicy=Qt.NoFocus, prefix="Tick: ", maximum=10000000, enabled=False)
        self.tickSpinBox.valueChanged.connect(self.tick)
        self.previousTickSpinBoxValue = 0

        self.instructionTextbox = QLineEdit(self, placeholderText="Click on the canvas.")
        self.instructionTextbox.setReadOnly(True)

        def toggleStartButton():
            if self.simulationTimer.isActive():
                self._stopSimulation()
            else:
                self._startSimulation()

        self.startButton = QPushButton(self, text="Start")
        self.startButton.clicked.connect(toggleStartButton)
        self.simulationTimer = QTimer(self, interval=int(1000 / self.TICKS_PER_SECOND))
        self.simulationTimer.timeout.connect(lambda: self.tickSpinBox.setValue(self.tickSpinBox.value() + 1))

        controlToolBar.addWidget(self.startButton)
        controlToolBar.addWidget(self.tickSpinBox)
        controlToolBar.addWidget(self.instructionTextbox)

    def __init__(self):
        """Initial game configuration."""
        super().__init__()

        self.simulation = None

        # default to sprint
        sprint = os.path.join(
            os.path.abspath(os.path.dirname(__file__)),
            "..",
            "maps",
            "sprint.txt",
        )
        if os.path.exists(sprint):
            print(sprint)
            self.load_simulation(path=sprint)
        else:
            self.load_simulation()

        self._prepareMenus()
        self._prepareTools()

        # current zoom level
        self.zoom = 0.01

        # the history of instructions/racers
        # the latter is just so we don't re-run the simulation every tick
        self.instructions = []
        # TODO: is copy-pasted
        self.racers = [dataclasses.replace(self.simulation.racer)]
        self.reached_goals = [list(self.simulation.reached_goals)]

        self.cursor: Instruction = Instruction()

        self.canvas = QFrame(self, minimumSize=QSize(800, 600))
        self.setCentralWidget(self.canvas)

        self.setWindowTitle('Asteracer')
        self.show()

    def paintEvent(self, event):
        """Paints the game."""

        def unzoom(x: float, y: Optional[float] = None) -> Union[float, Tuple[float, float]]:
            """Function for unzooming stuff. Works with radii and coordinates."""

            if y is None:
                return x * self.zoom

            return (
                self.canvas.width() / 2 + (-self.simulation.racer.x + x) * self.zoom,
                self.canvas.height() / 2 + (-self.simulation.racer.y + y) * self.zoom,
            )

        painter = QPainter(self)
        painter.setClipRegion(QRegion(0, 0, self.canvas.width(), self.canvas.height()))

        # background
        painter.setPen(QPen(Qt.gray, Qt.SolidLine))
        painter.setBrush(QBrush(Qt.gray, Qt.SolidPattern))
        painter.drawRect(0, 0, self.canvas.width(), self.canvas.height())

        # bounding box
        painter.setPen(QPen(Qt.black, Qt.SolidLine))
        painter.setBrush(QBrush(Qt.white, Qt.SolidPattern))

        p1 = unzoom(self.simulation.bounding_box.min_x, self.simulation.bounding_box.min_y)
        p2 = unzoom(self.simulation.bounding_box.max_x, self.simulation.bounding_box.max_y)

        painter.drawRect(QRectF(QPointF(*p1), QPointF(*p2)))

        # goals
        for goal, reached in zip(self.simulation.goals, self.simulation.reached_goals):
            if reached:
                painter.setPen(QPen(Qt.green, Qt.SolidLine))
                painter.setBrush(QBrush(Qt.green, Qt.SolidPattern))
            else:
                painter.setPen(QPen(Qt.red, Qt.SolidLine))
                painter.setBrush(QBrush(Qt.red, Qt.SolidPattern))

            x, y = unzoom(goal.x, goal.y)
            r = unzoom(goal.radius)
            painter.drawEllipse(QPointF(x, y), r, r)

        # path the racer took
        painter.setPen(QPen(Qt.gray, Qt.SolidLine))
        painter.setBrush(QBrush(Qt.gray, Qt.SolidPattern))

        path_points_r = unzoom(self.simulation.racer.radius / 2)
        painter.drawEllipse(QPointF(*unzoom(self.racers[0].x, self.racers[0].y)), path_points_r, path_points_r)
        painter.drawEllipse(QPointF(*unzoom(self.racers[-1].x, self.racers[-1].y)), path_points_r, path_points_r)
        for i in range(len(self.racers) - 1):
            painter.drawLine(
                QPointF(*unzoom(self.racers[i].x, self.racers[i].y)),
                QPointF(*unzoom(self.racers[i + 1].x, self.racers[i + 1].y)),
            )

        # asteroids
        painter.setPen(QPen(Qt.black, Qt.SolidLine))
        painter.setBrush(QBrush(Qt.black, Qt.SolidPattern))
        for asteroid in self.simulation.asteroids:
            x, y = unzoom(asteroid.x, asteroid.y)
            r = unzoom(asteroid.radius)
            painter.drawEllipse(QPointF(x, y), r, r)

        # racer info
        racer_x, racer_y = unzoom(self.simulation.racer.x, self.simulation.racer.y)
        racer_r = unzoom(self.simulation.racer.radius)

        # cursor + line to racer
        if self.cursor is not None:
            cursor_r = unzoom(self.simulation.racer.radius / 2)
            x = self.cursor.vx + self.canvas.width() / 2
            y = self.cursor.vy + self.canvas.height() / 2

            painter.setPen(QPen(Qt.red, Qt.SolidLine))
            painter.setBrush(QBrush(Qt.red, Qt.SolidPattern))

            painter.drawEllipse(QPointF(x, y), cursor_r, cursor_r)

            painter.drawLine(
                QPointF(x, y),
                QPointF(racer_x, racer_y),
            )

        # racer
        painter.setPen(QPen(Qt.black, Qt.SolidLine))
        painter.setBrush(QBrush(Qt.gray, Qt.SolidPattern))
        painter.drawEllipse(QPointF(racer_x, racer_y), racer_r, racer_r)

    def zoomOut(self):
        if self.zoom * self.ZOOM_MULTIPLIER <= self.MAX_ZOOM:
            self.zoom *= self.ZOOM_MULTIPLIER

    def zoomIn(self):
        if self.zoom / self.ZOOM_MULTIPLIER >= self.MIN_ZOOM:
            self.zoom /= self.ZOOM_MULTIPLIER

    def wheelEvent(self, event):
        if event.angleDelta().y() > 0:
            self.zoomOut()
        else:
            self.zoomIn()

        self.update()

    def mousePressEvent(self, event):
        self.mouseMoveEvent(event)

    def mouseMoveEvent(self, event):
        x = event.localPos().x()
        y = event.localPos().y()

        if 0 <= x <= self.canvas.width() and 0 <= y <= self.canvas.height():
            if self.cursor is None:
                self.tickSpinBox.setEnabled(True)

            self.cursor = Instruction(int(x - self.canvas.width() / 2), int(y - self.canvas.height() / 2))
            self.instructionTextbox.setText(f"{self.cursor.vx} {self.cursor.vy}")
            self.update()

    def _startSimulation(self):
        self.simulationTimer.start()
        self.startButton.setText("Stop")
        self.tickSpinBox.setEnabled(False)

    def _stopSimulation(self):
        self.simulationTimer.stop()
        self.startButton.setText("Start")
        self.tickSpinBox.setEnabled(True)

    def keyPressEvent(self, event):
        modifiers = QApplication.keyboardModifiers()

        # g/home -- to the start of the path
        if event.key() == Qt.Key_G or event.key() == Qt.Key_Home:
            self._stopSimulation()
            self.tickSpinBox.setValue(0)

        # G/end -- to the end of the path
        if (event.key() == Qt.Key_G and modifiers == Qt.ShiftModifier) or event.key() == Qt.Key_End:
            self._stopSimulation()
            self.tickSpinBox.setValue(len(self.instructions))

        # r -- restart the simulation
        if event.key() == Qt.Key_R:
            self._stopSimulation()
            self.tickSpinBox.setValue(0)
            self.simulation.restart()

            # the history of instructions/racers
            # the latter is just so we don't re-run the simulation every tick
            self.instructions = []
            self.racers = [dataclasses.replace(self.simulation.racer)]
            self.reached_goals = [list(self.simulation.reached_goals)]

            self.update()

        # - -- zoom out
        if event.key() == Qt.Key_Minus:
            self.zoomIn()
            self.update()

        # + -- zoom in
        if event.key() == Qt.Key_Plus:
            self.zoomOut()
            self.update()

        # h/l or </> -- back and forth
        if event.key() == Qt.Key_H or event.key() == Qt.Key_Left:
            self._stopSimulation()

            value = 1 if modifiers != Qt.ShiftModifier else self.TICKS_PER_SECOND
            self.tickSpinBox.setValue(self.tickSpinBox.value() - value)

        if event.key() == Qt.Key_L or event.key() == Qt.Key_Right:
            self._stopSimulation()

            value = 1 if modifiers != Qt.ShiftModifier else self.TICKS_PER_SECOND
            value = min(value, len(self.instructions) - self.tickSpinBox.value())

            self.tickSpinBox.setValue(self.tickSpinBox.value() + value)

    def tick(self):
        """Called when the tick spindown changes. Updates the simulation."""
        # current/previous tick (starting at 0, where no ticks have elapsed)
        prev_v = self.previousTickSpinBoxValue
        v = self.tickSpinBox.value()

        delta = v - prev_v

        instruction = self.cursor

        # if going forward and current instruction is the same as next from history, tick
        if delta == 1 and (len(self.instructions) < v or self.instructions[v - 1] != instruction):
            self.simulation.tick(instruction)

            self.instructions = self.instructions[:v - 1]
            self.racers = self.racers[:v]
            self.reached_goals = self.reached_goals[:v]

            # TODO: is copy-pasted
            self.instructions.append(instruction)
            self.racers.append(dataclasses.replace(self.simulation.racer))
            self.reached_goals.append(list(self.simulation.reached_goals))

        # else just replay
        else:
            self.simulation.racer = dataclasses.replace(self.racers[v])
            self.simulation.reached_goals = list(self.reached_goals[v])

            # when replaying and getting to the very end, make the instruction the last one
            # since it's yet to be determined
            if v == len(self.instructions):
                self.cursor = self.instructions[-1]
                self.instructionTextbox.setText(f"{self.instructions[-1].vx} {self.instructions[-1].vy}")

                self._stopSimulation()
            else:
                self.cursor = self.instructions[v]
                self.instructionTextbox.setText(f"{self.instructions[v].vx} {self.instructions[v].vy}")

        self.previousTickSpinBoxValue = v
        self.update()


app = QApplication(sys.argv)

ex = Asteracer()
sys.exit(app.exec_())
