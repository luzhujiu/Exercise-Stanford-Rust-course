#include <stdio.h>
int g1 = 1;
int g2 = 2;
int g3 = 3;
int g4 = 4;
int g5 = 5;
int g6 = 6;
void func1(int a, int b, int c, int d, int e, int f) {
    a = 1;
    b = 2;
    c = 3;
    d = 4;
    e = 5;
    f = 6;
    int g = 7;
    int h = 8;
    int i = 9;
    int j = 10;
    int k = 11;

    printf("%d\n",i);
}

int main() {
    int a = 0;
    int b = 1;
    int c = 2;
    int d = 3;
    int e = 4;
    int f = 5;
    func1(a, b, c,d,e,f);    
}
