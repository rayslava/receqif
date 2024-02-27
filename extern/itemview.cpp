#include "itemview.h"
#include "tvision/include/tvision/ttypes.h"

TPalette &TItemViewer::getPalette() const {
  static TPalette palette(cpTestView, sizeof(cpTestView) - 1);
  return palette;
}

const int maxLineLength = 256;

const char *const TItemViewer::name = "TItemViewer";
extern char text[100];

void TItemViewer::handleEvent(TEvent &event) {
  TScroller::handleEvent(event);

  switch (event.what) {
  case evKeyDown:
    switch (event.keyDown.keyCode) {
    case kbDown:
      if (selectedLine < fileLines->getCount() - 1)
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
  TAttrPair cNormal = getColor(1);
  TAttrPair cFrame = getColor(3);
  TAttrPair cSelected = getColor(2);
  TAttrPair cHeader = getColor(4);

  if (getState(sfFocused)) {
    setStyle(cHeader[0], slBold);
    setStyle(cFrame[0], slBold);
  }

  // Header
  b.moveChar(0, ' ', cNormal, size.x);
  b.moveChar(size.x - 1, BOX_SINGLE_VERTICAL, cFrame, 1);
  b.moveStr(0, fileName, getState(sfFocused) ? cHeader : cFrame);
  writeBuf(0, 0, (short)size.x, 1, b);
  // Line
  b.moveChar(0, ' ', cFrame, size.x);
  char line[128] = {0};
  memset(line, BOX_SINGLE_HORIZONTAL, size.x - 1);
  b.moveStr(0, line, cFrame);
  b.moveChar(size.x - 1, BOX_CROSS_SINGLE, cFrame, 1);
  writeBuf(0, 1, (short)size.x, 1, b);

  for (short i = 0; i < size.y; i++) {
    b.moveChar(0, ' ', cFrame, size.x);
    b.moveChar(size.x - 1, BOX_SINGLE_VERTICAL, cFrame, 1);
    if (delta.y + i < fileLines->getCount()) {
      p = (char *)(fileLines->at(delta.y + i));
      if (p)
        b.moveStr(0, p, i == selectedLine ? cSelected : cNormal);
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
