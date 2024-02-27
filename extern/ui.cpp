#include "app.h"

extern "C" int ui_main(char *hello_line) {
  THelloApp helloWorld;
  helloWorld.set_hello(hello_line);
  helloWorld.run();
  return 0;
}
