all:
	g++ -O0 -g3 -I tvision/include/ -fPIE -DBINARY ui.cpp -c -o ui.o
	g++ -O0 -g3 -L tvision/build -fPIE ui.o -Wl,-Bstatic -ltvision -Wl,-Bdynamic -lncursesw -ltinfow -lgpm -o ui

format:
	clang-format -i ui.cpp
