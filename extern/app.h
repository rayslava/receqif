#pragma once

#include "itemwindow.h"

#define Uses_TApplication
#define Uses_TEvent
#define Uses_TRect

#include <tvision/tv.h>

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

class THintStatusLine : public TStatusLine {
public:
  THintStatusLine(TRect r, TStatusDef &def) : TStatusLine(r, def) {}
  virtual const char *hint(ushort) override;
};
