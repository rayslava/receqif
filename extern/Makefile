CXX = g++
COMMONFLAGS := -I tvision/include/ -std=c++23 -fPIE -Wall -Wextra -Werror -pedantic -Wno-unknown-pragmas
#OPTFLAGS := -O0 -g3
OPTFLAGS := -Ofast -flto -fvisibility=hidden
LINKFLAGS := -L tvision/build -fPIE $(OPTFLAGS)
LIBS := -Wl,-Bstatic -ltvision -Wl,-Bdynamic -lncursesw -ltinfow -lgpm
SOURCES = $(wildcard *.cpp)

OBJECTS = $(SOURCES:.cpp=.o)

app: $(OBJECTS)
	g++ $(OPTFLAGS) $(LINKFLAGS) $(OBJECTS) $(LIBS) -o ui

%.o: %.cpp
	$(CXX) -c $(COMMONFLAGS) $(OPTFLAGS) $< -o $@

clean:
	rm -f $(OBJECTS)

format:
	clang-format -i *.h *.cpp
