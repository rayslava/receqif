#include "itemwindow.h"

#define Uses_TProgram
#define Uses_TDeskTop

#include <tvision/tv.h>

static short winNumber = 0;

TFrame *TItemWindow::initFrame(TRect r) { return new TCustomFrame(r); }

TItemWindow::TItemWindow(const char *fileName)
    : TWindowInit(&TItemWindow::initFrame),
      TWindow(TProgram::deskTop->getExtent(), fileName, winNumber++) {
  options |= ofTileable;

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

TPalette &TItemWindow::getPalette() const {
  static TPalette palette(cpItemWindow, sizeof(cpItemWindow) - 1);
  return palette;
}
