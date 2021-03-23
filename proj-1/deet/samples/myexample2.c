#include <stdio.h>
int global = 5;
void func1(int a, int b, int c) {
    a += 1;
    b += 1;
    c += 1;
    int d = a + b + c;

    printf("%d\n",d);
}

int main() {
    int a = 0;
    int b = 1;
    int c = 2;
    int e = 10;
    func1(a, b, c);    
}
