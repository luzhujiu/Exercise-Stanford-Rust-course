#include <stdio.h>

void func1(int a) {
    printf("Calling func1\n");
    printf("%d\n", a);
}

int main() {
    func1(1);
    func1(2);
    func1(3);
}
