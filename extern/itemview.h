#pragma once

#define Uses_TWindow
#define Uses_TCollection
#define Uses_TScroller
#define Uses_TEvent
#define Uses_TKeys
#define Uses_TProgram
#define Uses_TStatusLine
#define Uses_TStreamableClass
#define Uses_opstream

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
const char BOX_SINGLE_HORIZONTAL_TO_SINGLE_VERTICAL_TOP = '\xC2'; // ┬

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
