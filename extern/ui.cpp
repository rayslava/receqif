#define Uses_TKeys
#define Uses_TApplication
#define Uses_TEvent
#define Uses_TRect
#define Uses_TDialog
#define Uses_TStaticText
#define Uses_TButton
#define Uses_TMenuBar
#define Uses_TSubMenu
#define Uses_TMenuItem
#define Uses_TStatusLine
#define Uses_TStatusItem
#define Uses_TStatusDef
#define Uses_TScroller
#define Uses_TDeskTop
#define Uses_TCollection
#define Uses_TScroller
#define Uses_TWindow
#define Uses_ipstream
#define Uses_opstream
#define Uses_TStreamableClass
#define Uses_TFrame

#include <memory>
#include <tvision/tv.h>

// Single Line Frame Characters
const char BOX_SINGLE_HORIZONTAL = '\xC4'; // ─
const char BOX_SINGLE_VERTICAL = '\xB3';   // │

// Double Line Frame Characters
const char BOX_DOUBLE_HORIZONTAL = '\xCD'; // ═
const char BOX_DOUBLE_VERTICAL = '\xBA';   // ║

// Junctions: Single Horizontal to Double Vertical
const char BOX_SINGLE_HORIZONTAL_TO_DOUBLE_VERTICAL_TOP =
    '\xD1'; // ╤ (Approximation)
const char BOX_SINGLE_HORIZONTAL_TO_DOUBLE_VERTICAL_BOTTOM =
    '\xCF'; // ╧ (Approximation)

// Junctions: Double Horizontal to Single Vertical
const char BOX_DOUBLE_HORIZONTAL_TO_SINGLE_VERTICAL_LEFT =
    '\xC3'; // ╟ (Approximation)
const char BOX_DOUBLE_HORIZONTAL_TO_SINGLE_VERTICAL_RIGHT =
    '\xB4'; // ╢ (Approximation)

// Cross junctions
const char BOX_CROSS_SINGLE = '\xC5'; // ┼
const char BOX_CROSS_DOUBLE =
    '\xCE'; // ╬ (No direct equivalent, used closest match)
const char BOX_CROSS_SINGLE_TO_DOUBLE = '\xD8'; // ╪ (Approximation)
const char BOX_CROSS_DOUBLE_TO_SINGLE = '\xD7'; // ╫ (Approximation)

// Corners
const char BOX_CORNER_TOP_LEFT_SINGLE = '\xDA';     // ┌
const char BOX_CORNER_TOP_RIGHT_SINGLE = '\xBF';    // ┐
const char BOX_CORNER_BOTTOM_LEFT_SINGLE = '\xC0';  // └
const char BOX_CORNER_BOTTOM_RIGHT_SINGLE = '\xD9'; // ┘

const char BOX_CORNER_TOP_LEFT_DOUBLE = '\xC9';     // ╔
const char BOX_CORNER_TOP_RIGHT_DOUBLE = '\xBB';    // ╗
const char BOX_CORNER_BOTTOM_LEFT_DOUBLE = '\xC8';  // ╚
const char BOX_CORNER_BOTTOM_RIGHT_DOUBLE = '\xBC'; // ╝

static char text[100] = "iniytial";

class TItemCollection : public TCollection {

public:
  TItemCollection(short lim, short delta) : TCollection(lim, delta) {}
  virtual void freeItem(void *p) { delete[] (char *)p; }

private:
  virtual void *readItem(ipstream &) { return 0; }
  virtual void writeItem(void *, opstream &) {}
};

class TItemViewer : public TScroller {

  int selectedLine = 0;

public:
  enum class ViewedColumn { Items, Categories, Weights };

  char *fileName;
  TCollection *fileLines;
  Boolean isValid;
  TItemViewer(const TRect &bounds, TScrollBar *aHScrollBar,
              TScrollBar *aVScrollBar, const ViewedColumn &col);
  ~TItemViewer();
  TItemViewer(StreamableInit) : TScroller(streamableInit){};
  void draw();
  void setState(ushort aState, Boolean enable);
  void scrollDraw();
  Boolean valid(ushort command) const;
  virtual void handleEvent(TEvent &event) override;
  int findSel(TPoint p);
  virtual TPalette &getPalette() const override;

private:
  virtual const char *streamableName() const { return name; }

protected:
  virtual void write(opstream &);
  virtual void *read(ipstream &);

public:
  static const char *const name;
  static TStreamable *build();
};

#define cpTestView "\x6\x7\x2\x9"

TPalette &TItemViewer::getPalette() const {
  static TPalette palette(cpTestView, sizeof(cpTestView) - 1);
  return palette;
}

class TItemWindow : public TWindow {
  TItemViewer *itemViewer, *catViewer, *weightViewer;
  const TRect itemViewerBounds() const {
    auto bounds = getExtent();
    TRect r(bounds.a.x, bounds.a.y, bounds.b.x / 3, bounds.b.y);
    r.grow(-1, -1);
    return r;
  }

  const TRect catViewerBounds() const {
    auto bounds = getExtent();
    TRect r(bounds.b.x / 3 - 2, bounds.a.y, 2 * bounds.b.x / 3 - 1, bounds.b.y);
    r.grow(-1, -1);
    return r;
  }

  const TRect weightViewerBounds() const {
    auto bounds = getExtent();
    TRect r(2 * bounds.b.x / 3 - 3, bounds.a.y, bounds.b.x, bounds.b.y);
    r.grow(-1, -1);
    return r;
  }

public:
  TItemWindow(const char *fileName);
  static TFrame *initFrame(TRect r);
  void draw() override;
  virtual void handleEvent(TEvent &event) override;
  virtual TPalette &getPalette() const override;

protected:
  virtual void sizeLimits(TPoint &min, TPoint &max) override {
    TWindow::sizeLimits(min, max);
    min.x = size.x / 2 + 10;
  };
};
#define cpItemWindow "\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x06"

TPalette &TItemWindow::getPalette() const {
  static TPalette palette(cpItemWindow, sizeof(cpItemWindow) - 1);
  return palette;
}

const int maxLineLength = 256;

const char *const TItemViewer::name = "TItemViewer";

void TItemViewer::handleEvent(TEvent &event) {
  TScroller::handleEvent(event);

  switch (event.what) {
  case evKeyDown:
    switch (event.keyDown.keyCode) {
    case kbDown:
      if (selectedLine < fileLines->getCount() - 1) // -2 is header for now
        selectedLine++;
      clearEvent(event);
      break;
    case kbUp:
      if (selectedLine > 0)
        selectedLine--;
      clearEvent(event);
      break;
    }
    break;
  case evMouseDown:
    TPoint mouse = makeLocal(event.mouse.where);
    int i = findSel(mouse);
    if (i != -1)
      selectedLine = i;
    clearEvent(event);
    break;
  }
  sprintf(text, "%s: %d", fileName, selectedLine);
  TProgram::application->statusLine->update();
  draw();
  TProgram::application->statusLine->draw();
}

int TItemViewer::findSel(TPoint p) {
  TRect r = getExtent();
  if (!r.contains(p))
    return -1;
  else {
    int s = p.y - 2;
    if (s >= fileLines->getCount())
      return -1;
    else
      return s;
  }
}

TItemViewer::TItemViewer(const TRect &bounds, TScrollBar *aHScrollBar,
                         TScrollBar *aVScrollBar, const ViewedColumn &col)
    : TScroller(bounds, aHScrollBar, aVScrollBar) {
  if (col == ViewedColumn::Categories)
    growMode = gfGrowHiX | gfGrowHiY;
  else
    growMode = gfGrowHiY;

  isValid = True;
  fileLines = new TItemCollection(5, 5);
  switch (col) {
  case ViewedColumn::Items:
    fileName = newStr("Items");
    break;
  case ViewedColumn::Categories:
    fileName = newStr("Categories");
    break;
  case ViewedColumn::Weights:
    fileName = newStr("Weights");
    break;
  }

  fileLines->insert(newStr("1"));
  fileLines->insert(newStr("2"));
  fileLines->insert(newStr("3"));
  fileLines->insert(newStr("4"));
  fileLines->insert(newStr("5"));
  fileLines->insert(newStr("6"));
  fileLines->insert(newStr("7"));
}

TItemViewer::~TItemViewer() { destroy(fileLines); }

void TItemViewer::draw() {
  TDrawBuffer b;
  char *p;
  TAttrPair c = getColor(1);
  TAttrPair cFrame = getColor(3);
  TAttrPair cSelected = getColor(2);
  TAttrPair cHeader = getColor(4);

  // Header
  b.moveChar(0, ' ', c, size.x);
  b.moveChar(size.x - 1, BOX_SINGLE_VERTICAL, cFrame, 1);
  b.moveStr(0, fileName, getState(sfFocused) ? cHeader : c);
  writeBuf(0, 0, (short)size.x, 1, b);
  // Line
  b.moveChar(0, ' ', cFrame, size.x);
  char line[size.x] = {0};
  memset(line, BOX_SINGLE_HORIZONTAL, size.x - 1);
  b.moveStr(0, line, cFrame);
  b.moveChar(size.x - 1, BOX_CROSS_SINGLE, cFrame, 1);
  writeBuf(0, 1, (short)size.x, 1, b);

  for (short i = 0; i < size.y; i++) {
    b.moveChar(0, ' ', c, size.x);
    b.moveChar(size.x - 1, BOX_SINGLE_VERTICAL, c, 1);
    if (delta.y + i < fileLines->getCount()) {
      p = (char *)(fileLines->at(delta.y + i));
      if (p)
        b.moveStr(0, p, i == selectedLine ? cSelected : c);
    }
    writeBuf(0, i + 2, (short)size.x, 1, b);
  }
}

void TItemViewer::scrollDraw() {
  TScroller::scrollDraw();
  draw();
}

void TItemViewer::setState(ushort aState, Boolean enable) {
  TScroller::setState(aState, enable);
  if (enable && (aState & sfExposed))
    setLimit(limit.x, limit.y);
}

Boolean TItemViewer::valid(ushort) const { return isValid; }

void *TItemViewer::read(ipstream &is) {
  char *fName = NULL;
  TScroller::read(is);
  delete fName;
  return this;
}

void TItemViewer::write(opstream &os) {
  TScroller::write(os);
  os.writeString(fileName);
}

TStreamable *TItemViewer::build() { return new TItemViewer(streamableInit); }

TStreamableClass RItemView(TItemViewer::name, TItemViewer::build,
                           __DELTA(TItemViewer));

static short winNumber = 0;

class TCustomFrame : public TFrame {
public:
  using TFrame::TFrame; // Inherit constructor

  virtual void draw() override {
    TFrame::draw(); // Call base class draw
    TDrawBuffer b;
    TAttrPair cNormal = getColor(4);
    int fstSep = size.x / 3 - 2;
    int sndSep = 2 * size.x / 3 - 3;

    b.moveChar(0, ' ', cNormal, 1);
    b.putChar(0, BOX_SINGLE_HORIZONTAL_TO_DOUBLE_VERTICAL_TOP);
    writeLine(fstSep, 0, 1, 1, b);
    writeLine(sndSep, 0, 1, 1, b);
  }
};

TFrame *TItemWindow::initFrame(TRect r) { return new TCustomFrame(r); }

TItemWindow::TItemWindow(const char *fileName)
    : TWindow(TProgram::deskTop->getExtent(), fileName, winNumber++),
      TWindowInit(&TItemWindow::initFrame) {
  options |= ofTileable;
  auto bounds = getExtent();

  itemViewer = new TItemViewer(
      itemViewerBounds(), standardScrollBar(sbHorizontal | sbHandleKeyboard),
      standardScrollBar(sbVertical | sbHandleKeyboard),
      TItemViewer::ViewedColumn::Items);
  insert(itemViewer);

  catViewer = new TItemViewer(
      catViewerBounds(), standardScrollBar(sbHorizontal | sbHandleKeyboard),
      standardScrollBar(sbVertical | sbHandleKeyboard),
      TItemViewer::ViewedColumn::Categories);
  insert(catViewer);

  weightViewer = new TItemViewer(
      weightViewerBounds(), standardScrollBar(sbHorizontal | sbHandleKeyboard),
      standardScrollBar(sbVertical | sbHandleKeyboard),
      TItemViewer::ViewedColumn::Weights);
  insert(weightViewer);
}

void TItemWindow::draw() {
  itemViewer->changeBounds(itemViewerBounds());
  catViewer->changeBounds(catViewerBounds());
  weightViewer->changeBounds(weightViewerBounds());
  TWindow::draw();
}

void TItemWindow::handleEvent(TEvent &event) { TWindow::handleEvent(event); }

const int GreetThemCmd = 100;
const int CallListCmd = 101;

class THelloApp : public TApplication {

public:
  THelloApp();

  virtual void handleEvent(TEvent &event);
  static TMenuBar *initMenuBar(TRect);
  static TStatusLine *initStatusLine(TRect);
  void set_hello(const std::string &new_line) { hello_line = new_line; }
  virtual void idle() override;

private:
  std::string hello_line;
  void greetingBox();
};

THelloApp::THelloApp()
    : TProgInit(&THelloApp::initStatusLine, &THelloApp::initMenuBar,
                &THelloApp::initDeskTop) {}

void THelloApp::greetingBox() {
  TDialog *d = new TDialog(TRect(25, 5, 55, 16), "Hello, World!");

  d->insert(new TStaticText(TRect(3, 5, 15, 6), hello_line.c_str()));
  d->insert(new TButton(TRect(16, 2, 28, 4), "Terrific", cmCancel, bfNormal));
  d->insert(new TButton(TRect(16, 4, 28, 6), "Ok", cmCancel, bfNormal));
  d->insert(new TButton(TRect(16, 6, 28, 8), "Lousy", cmCancel, bfNormal));
  d->insert(new TButton(TRect(16, 8, 28, 10), "Cancel", cmCancel, bfNormal));

  deskTop->execView(d);
  destroy(d);
}

void THelloApp::handleEvent(TEvent &event) {
  TApplication::handleEvent(event);
  if (event.what == evCommand) {
    switch (event.message.command) {
    case GreetThemCmd:
      greetingBox();
      clearEvent(event);
      break;
    case CallListCmd:
      if (TView *w = validView(new TItemWindow("test")))
        deskTop->insert(w);
      clearEvent(event);
      break;
    default:
      break;
    }
  }
}

TMenuBar *THelloApp::initMenuBar(TRect r) {

  r.b.y = r.a.y + 1;

  return new TMenuBar(
      r, *new TSubMenu("~F~ile", kbAltH) +
             *new TMenuItem("~G~reeting...", GreetThemCmd, kbAltG) +
             *new TMenuItem("~L~ist...", CallListCmd, kbAltL) + newLine() +
             *new TMenuItem("E~x~it", cmQuit, cmQuit, hcNoContext, "Alt-X"));
}

class HintStatusLine : public TStatusLine {
public:
  HintStatusLine(TRect r, TStatusDef &def) : TStatusLine(r, def) {}
  virtual const char *hint(ushort) override;
};

const char *HintStatusLine::hint(ushort aHelpCtx) { return text; }

TStatusLine *THelloApp::initStatusLine(TRect r) {
  r.a.y = r.b.y - 1;

  return new HintStatusLine(
      r, *new TStatusDef(0, 50) +
             *new TStatusItem("~F10~ Menu", kbF10, cmMenu) +
             *new TStatusItem("~Alt-X~ Exit", kbAltX, cmQuit) +

             *new TStatusDef(50, 0xFFFF) + *new TStatusItem(0, kbF10, cmMenu) +
             *new TStatusItem("~F1~ Help", kbF1, cmHelp));
}

void THelloApp::idle() {
  if (statusLine != 0)
    statusLine->update();

  if (commandSetChanged == True) {
    message(this, evBroadcast, cmCommandSetChanged, 0);
    commandSetChanged = False;
  }
}

#ifndef BINARY
extern "C" int ui_main(char *hello_line) {
#else
int main() {
  const char *hello_line = "test line";
#endif
  THelloApp helloWorld;
  helloWorld.set_hello(hello_line);
  helloWorld.run();
  return 0;
}
