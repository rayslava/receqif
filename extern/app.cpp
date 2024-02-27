#include "app.h"

#define Uses_TDialog
#define Uses_TKeys
#define Uses_TDeskTop
#define Uses_TButton
#define Uses_TStaticText
#define Uses_TStatusLine
#define Uses_TStatusDef
#define Uses_TStatusItem
#define Uses_TSubMenu
#define Uses_TMenuBar

#include <tvision/tv.h>

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

char text[100] = "init";

const char *THintStatusLine::hint(ushort) { return text; }

TStatusLine *THelloApp::initStatusLine(TRect r) {
  r.a.y = r.b.y - 1;

  return new THintStatusLine(
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
