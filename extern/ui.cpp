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

#include <string>
#include <tvision/tv.h>

class TItemCollection : public TCollection {

public:
  TItemCollection(short lim, short delta) : TCollection(lim, delta) {}
  virtual void freeItem(void *p) { delete[](char *) p; }

private:
  virtual void *readItem(ipstream &) { return 0; }
  virtual void writeItem(void *, opstream &) {}
};

class TItemViewer : public TScroller {

public:
  char *fileName;
  TCollection *fileLines;
  Boolean isValid;
  TItemViewer(const TRect &bounds, TScrollBar *aHScrollBar,
              TScrollBar *aVScrollBar, Boolean left);
  ~TItemViewer();
  TItemViewer(StreamableInit) : TScroller(streamableInit){};
  void draw();
  void setState(ushort aState, Boolean enable);
  void scrollDraw();
  Boolean valid(ushort command);

private:
  virtual const char *streamableName() const { return name; }

protected:
  virtual void write(opstream &);
  virtual void *read(ipstream &);

public:
  static const char *const name;
  static TStreamable *build();
};

class TItemWindow : public TWindow {

public:
  TItemWindow(const char *fileName);
protected:
  virtual void sizeLimits(TPoint& min, TPoint& max) override {
    TWindow::sizeLimits(min, max);
    min.x = size.x/2+10;
  };

};

const int maxLineLength = 256;

const char *const TItemViewer::name = "TItemViewer";

TItemViewer::TItemViewer(const TRect &bounds, TScrollBar *aHScrollBar,
                         TScrollBar *aVScrollBar, Boolean left)
    : TScroller(bounds, aHScrollBar, aVScrollBar) {
  if (left)
    growMode = gfGrowHiY;
  else
    growMode = gfGrowHiX | gfGrowHiY;

  isValid = True;
  fileName = 0;
  fileLines = new TItemCollection(5, 5);
  fileLines->insert(newStr(left ? "Items" : "Categories"));
}

TItemViewer::~TItemViewer() { destroy(fileLines); }

void TItemViewer::draw() {
  char *p;

  ushort c = getColor(0x0301);

  for (short i = 0; i < size.y; i++) {
    TDrawBuffer b;
    b.moveChar(0, ' ', c, size.x);

    if (delta.y + i < fileLines->getCount()) {
      p = (char *)(fileLines->at(delta.y + i));
      if (p)
        b.moveStr(0, p, c, (short)size.x, (short)delta.x);
    }
    writeBuf(0, i, (short)size.x, 1, b);
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

Boolean TItemViewer::valid(ushort) { return isValid; }

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

TItemWindow::TItemWindow(const char *fileName)
    : TWindow(TProgram::deskTop->getExtent(), fileName, winNumber++),
      TWindowInit(&TItemWindow::initFrame) {
  options |= ofTileable;
  auto bounds = getExtent();
  TRect r(bounds.a.x, bounds.a.y, bounds.b.x / 2 + 1, bounds.b.y);
  r.grow(-1, -1);
  insert(new TItemViewer(r, standardScrollBar(sbHorizontal | sbHandleKeyboard),
                         standardScrollBar(sbVertical | sbHandleKeyboard),
                         True));
  r = TRect(bounds.b.x / 2, bounds.a.y, bounds.b.x, bounds.b.y);
  r.grow(-1, -1);
  insert(new TItemViewer(r, standardScrollBar(sbHorizontal | sbHandleKeyboard),
                         standardScrollBar(sbVertical | sbHandleKeyboard),
                         False));
}

const int GreetThemCmd = 100;
const int CallListCmd = 101;

class THelloApp : public TApplication {

public:
  THelloApp();

  virtual void handleEvent(TEvent &event);
  static TMenuBar *initMenuBar(TRect);
  static TStatusLine *initStatusLine(TRect);
  void set_hello(const std::string &new_line) { hello_line = new_line; }

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

TStatusLine *THelloApp::initStatusLine(TRect r) {
  r.a.y = r.b.y - 1;
  return new TStatusLine(r,
                         *new TStatusDef(0, 0xFFFF) +
                             *new TStatusItem("~Alt-X~ Exit", kbAltX, cmQuit) +
                             *new TStatusItem(0, kbF10, cmMenu));
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
