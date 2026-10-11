#include <stdio.h>
#include <string.h>

#include "autoc.h"

int main() {
    const char *src = "int main() {";
    const char *const *args = {};
    autoc_Doc result = autoc_Doc_new("main.cpp", strlen(src), src, 0, args);

    if (result.translation_unit == NULL) {
        fprintf(stderr, "FAILED!\n");
    } else {
        printf("SUCCESS!\n");
    }
}
