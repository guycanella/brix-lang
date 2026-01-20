#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// ==========================================
// SECTION 1: MATRIX (v0.3)
// ==========================================

typedef struct {
  long rows;
  long cols;
  double *data;
} Matrix;

Matrix *matrix_new(long rows, long cols) {
  Matrix *m = (Matrix *)malloc(sizeof(Matrix));
  m->rows = rows;
  m->cols = cols;
  m->data = (double *)malloc(rows * cols * sizeof(double));
  return m;
}

Matrix *read_csv(char *filename) {
  FILE *file = fopen(filename, "r");
  if (!file) {
    printf("Erro: Nao foi possivel abrir o arquivo '%s'\n", filename);
    exit(1);
  }

  long rows = 0;
  long cols = 0;
  char buffer[4096];

  if (fgets(buffer, sizeof(buffer), file)) {
    rows++;
    cols = 1;
    char *ptr = buffer;
    while (*ptr) {
      if (*ptr == ',')
        cols++;
      ptr++;
    }
  }

  while (fgets(buffer, sizeof(buffer), file)) {
    if (strlen(buffer) > 1)
      rows++;
  }

  rewind(file);

  Matrix *m = matrix_new(rows, cols);

  long r = 0;
  long c = 0;
  while (fgets(buffer, sizeof(buffer), file) && r < rows) {
    char *token = strtok(buffer, ",");
    c = 0;
    while (token && c < cols) {
      m->data[r * cols + c] = atof(token);
      token = strtok(NULL, ",");
      c++;
    }
    r++;
  }

  fclose(file);
  return m;
}

// ==========================================
// SECTION 2: STRINGS (v0.4)
// ==========================================

typedef struct {
  long len;
  char *data;
} BrixString;

// Create a new string copying a C literal (e.g: "ola")
BrixString *str_new(char *raw_text) {
  BrixString *s = (BrixString *)malloc(sizeof(BrixString));
  if (raw_text == NULL) {
    s->len = 0;
    s->data = (char *)malloc(1);
    s->data[0] = '\0';
  } else {
    s->len = strlen(raw_text);
    s->data = (char *)malloc(s->len + 1); // +1 para o \0
    strcpy(s->data, raw_text);
  }
  return s;
}

// Concatenate two strings (a + b)
BrixString *str_concat(BrixString *a, BrixString *b) {
  BrixString *s = (BrixString *)malloc(sizeof(BrixString));
  s->len = a->len + b->len;

  // Allocate space for both strings
  s->data = (char *)malloc(s->len + 1);

  strcpy(s->data, a->data);
  strcat(s->data, b->data);

  return s;
}

// Compare equality (a == b) -> Returns 1 (true) or 0 (false)
long str_eq(BrixString *a, BrixString *b) {
  if (a->len != b->len)
    return 0; // Tamanhos diferentes = diferente
  return (strcmp(a->data, b->data) == 0) ? 1 : 0;
}

// Helper to print Brix string (since printf expects char*, not struct)
void print_brix_string(BrixString *s) {
  if (s && s->data) {
    printf("%s", s->data);
  } else {
    printf("(null)");
  }
}