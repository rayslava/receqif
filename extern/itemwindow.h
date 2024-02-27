#pragma once

#include "itemview.h"

#define Uses_TWindow
#define Uses_TRect
#define Uses_TEvent
#define Uses_TFrame

#include <tvision/tv.h>

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

class TCustomFrame : public TFrame {
public:
  using TFrame::TFrame; // Inherit constructor

  virtual void draw() override {
    TFrame::draw(); // Call base class draw
    TDrawBuffer b;
    TAttrPair cNormal = getState(sfActive) ? getColor(3) : getColor(1);

    if (getState(sfDragging))
      cNormal = getColor(5);

    int fstSep = size.x / 3 - 2;
    int sndSep = 2 * size.x / 3 - 3;

    b.moveChar(0, ' ', cNormal, 1);
    b.putChar(0, (getState(sfActive) && !getState(sfDragging))
                     ? BOX_SINGLE_HORIZONTAL_TO_DOUBLE_VERTICAL_TOP
                     : BOX_SINGLE_HORIZONTAL_TO_SINGLE_VERTICAL_TOP);
    writeLine(fstSep, 0, 1, 1, b);
    writeLine(sndSep, 0, 1, 1, b);
  }
};

#define cpItemWindow "\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x06"
