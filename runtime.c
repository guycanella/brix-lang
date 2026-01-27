#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

// ==========================================
// SECTION 0: COMPLEX NUMBERS (v1.0)
// ==========================================

typedef struct {
    double real;
    double imag;
} Complex;

// === Constructors ===

Complex complex_new(double real, double imag) {
    return (Complex){ real, imag };
}

// === Operators ===

Complex complex_add(Complex z1, Complex z2) {
    return (Complex){ z1.real + z2.real, z1.imag + z2.imag };
}

Complex complex_sub(Complex z1, Complex z2) {
    return (Complex){ z1.real - z2.real, z1.imag - z2.imag };
}

Complex complex_mul(Complex z1, Complex z2) {
    double real = z1.real * z2.real - z1.imag * z2.imag;
    double imag = z1.real * z2.imag + z1.imag * z2.real;
    return (Complex){ real, imag };
}

Complex complex_div(Complex z1, Complex z2) {
    double denom = z2.real * z2.real + z2.imag * z2.imag;
    if (denom == 0.0) {
        fprintf(stderr, "Error: Division by zero (complex)\n");
        exit(1);
    }
    double real = (z1.real * z2.real + z1.imag * z2.imag) / denom;
    double imag = (z1.imag * z2.real - z1.real * z2.imag) / denom;
    return (Complex){ real, imag };
}

// === Power Functions ===

Complex complex_powi(Complex z, int n) {
    if (n == 0) return (Complex){ 1.0, 0.0 };
    if (n == 1) return z;
    if (n < 0) {
        Complex pos_pow = complex_powi(z, -n);
        return complex_div((Complex){1.0, 0.0}, pos_pow);
    }

    // Binary exponentiation
    Complex result = { 1.0, 0.0 };
    Complex base = z;

    while (n > 0) {
        if (n % 2 == 1) {
            result = complex_mul(result, base);
        }
        base = complex_mul(base, base);
        n /= 2;
    }

    return result;
}

double complex_abs(Complex z);  // Forward declaration

Complex complex_powf(Complex z, double exp) {
    double r = complex_abs(z);
    double theta = atan2(z.imag, z.real);

    double new_r = pow(r, exp);
    double new_theta = theta * exp;

    return (Complex){
        new_r * cos(new_theta),
        new_r * sin(new_theta)
    };
}

Complex complex_exp(Complex z);  // Forward declaration
Complex complex_log(Complex z);  // Forward declaration

Complex complex_pow(Complex base, Complex exp) {
    // z1^z2 = exp(z2 * log(z1))
    Complex log_base = complex_log(base);
    Complex product = complex_mul(exp, log_base);
    return complex_exp(product);
}

// === Basic Properties ===

double complex_real(Complex z) {
    return z.real;
}

double complex_imag(Complex z) {
    return z.imag;
}

Complex complex_conj(Complex z) {
    return (Complex){ z.real, -z.imag };
}

double complex_abs(Complex z) {
    return sqrt(z.real * z.real + z.imag * z.imag);
}

double complex_abs2(Complex z) {
    return z.real * z.real + z.imag * z.imag;
}

double complex_angle(Complex z) {
    return atan2(z.imag, z.real);
}

// === Transcendental Functions ===

Complex complex_exp(Complex z) {
    double exp_real = exp(z.real);
    return (Complex){
        exp_real * cos(z.imag),
        exp_real * sin(z.imag)
    };
}

Complex complex_log(Complex z) {
    return (Complex){
        log(complex_abs(z)),
        complex_angle(z)
    };
}

Complex complex_sqrt(Complex z) {
    double r = complex_abs(z);
    double theta = complex_angle(z);
    double sqrt_r = sqrt(r);
    return (Complex){
        sqrt_r * cos(theta / 2.0),
        sqrt_r * sin(theta / 2.0)
    };
}

// === Trigonometric Functions ===

Complex complex_csin(Complex z) {
    return (Complex){
        sin(z.real) * cosh(z.imag),
        cos(z.real) * sinh(z.imag)
    };
}

Complex complex_ccos(Complex z) {
    return (Complex){
        cos(z.real) * cosh(z.imag),
        -sin(z.real) * sinh(z.imag)
    };
}

Complex complex_ctan(Complex z) {
    return complex_div(complex_csin(z), complex_ccos(z));
}

// === Hyperbolic Functions ===

Complex complex_csinh(Complex z) {
    return (Complex){
        sinh(z.real) * cos(z.imag),
        cosh(z.real) * sin(z.imag)
    };
}

Complex complex_ccosh(Complex z) {
    return (Complex){
        cosh(z.real) * cos(z.imag),
        sinh(z.real) * sin(z.imag)
    };
}

Complex complex_ctanh(Complex z) {
    return complex_div(complex_csinh(z), complex_ccosh(z));
}

// === Utility Functions ===

char* complex_to_string(Complex z) {
    char* buffer = malloc(100);
    if (z.imag >= 0) {
        snprintf(buffer, 100, "%.6g+%.6gi", z.real, z.imag);
    } else {
        snprintf(buffer, 100, "%.6g%.6gi", z.real, z.imag);  // minus sign included in imag
    }
    return buffer;
}

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
// SECTION 1.5: INTMATRIX (v0.6)
// ==========================================

typedef struct {
  long rows;
  long cols;
  long *data;  // i64* instead of double*
} IntMatrix;

IntMatrix *intmatrix_new(long rows, long cols) {
  IntMatrix *m = (IntMatrix *)malloc(sizeof(IntMatrix));
  m->rows = rows;
  m->cols = cols;
  m->data = (long *)calloc(rows * cols, sizeof(long));  // calloc zeros memory
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

// ==========================================
// SECTION 3: STATISTICS (v0.7)
// ==========================================

#include <math.h>

// Sum of all elements in matrix
double brix_sum(Matrix *m) {
  double sum = 0.0;
  long total = m->rows * m->cols;
  for (long i = 0; i < total; i++) {
    sum += m->data[i];
  }
  return sum;
}

// Mean (average) of all elements
double brix_mean(Matrix *m) {
  long total = m->rows * m->cols;
  if (total == 0) return 0.0;
  return brix_sum(m) / (double)total;
}

// Comparison function for qsort
static int compare_doubles(const void *a, const void *b) {
  double da = *(const double *)a;
  double db = *(const double *)b;
  return (da > db) - (da < db);
}

// Median (middle value when sorted)
double brix_median(Matrix *m) {
  long total = m->rows * m->cols;
  if (total == 0) return 0.0;

  // Copy data to temporary array
  double *temp = (double *)malloc(total * sizeof(double));
  memcpy(temp, m->data, total * sizeof(double));

  // Sort the array
  qsort(temp, total, sizeof(double), compare_doubles);

  double result;
  if (total % 2 == 0) {
    // Even: average of two middle elements
    result = (temp[total/2 - 1] + temp[total/2]) / 2.0;
  } else {
    // Odd: middle element
    result = temp[total/2];
  }

  free(temp);
  return result;
}

// Variance (average of squared differences from mean)
double brix_variance(Matrix *m) {
  long total = m->rows * m->cols;
  if (total == 0) return 0.0;

  double mean = brix_mean(m);
  double sum_sq_diff = 0.0;

  for (long i = 0; i < total; i++) {
    double diff = m->data[i] - mean;
    sum_sq_diff += diff * diff;
  }

  return sum_sq_diff / (double)total;
}

// Standard deviation (square root of variance)
double brix_std(Matrix *m) {
  return sqrt(brix_variance(m));
}

// ==========================================
// SECTION 4: LINEAR ALGEBRA (v0.7)
// ==========================================

// Identity matrix: create nxn matrix with 1s on diagonal, 0s elsewhere
Matrix *brix_eye(long n) {
  Matrix *result = matrix_new(n, n);

  for (long i = 0; i < n; i++) {
    for (long j = 0; j < n; j++) {
      result->data[i * n + j] = (i == j) ? 1.0 : 0.0;
    }
  }

  return result;
}

// Transpose: swap rows and columns
Matrix *brix_tr(Matrix *m) {
  Matrix *result = matrix_new(m->cols, m->rows);

  for (long i = 0; i < m->rows; i++) {
    for (long j = 0; j < m->cols; j++) {
      result->data[j * m->rows + i] = m->data[i * m->cols + j];
    }
  }

  return result;
}

// Determinant (simple implementation for small matrices)
// Note: This is a basic implementation. For production, use LAPACK.
double brix_det(Matrix *m) {
  if (m->rows != m->cols) {
    printf("Error: Determinant requires square matrix\n");
    return 0.0;
  }

  long n = m->rows;

  // Base cases
  if (n == 1) {
    return m->data[0];
  }

  if (n == 2) {
    return m->data[0] * m->data[3] - m->data[1] * m->data[2];
  }

  // For 3x3 and larger: use LU decomposition approach
  // Create a copy for in-place modification
  double *a = (double *)malloc(n * n * sizeof(double));
  memcpy(a, m->data, n * n * sizeof(double));

  double det = 1.0;

  // Gaussian elimination with partial pivoting
  for (long i = 0; i < n; i++) {
    // Find pivot
    long pivot = i;
    for (long j = i + 1; j < n; j++) {
      if (fabs(a[j * n + i]) > fabs(a[pivot * n + i])) {
        pivot = j;
      }
    }

    // Swap rows if needed
    if (pivot != i) {
      for (long k = 0; k < n; k++) {
        double temp = a[i * n + k];
        a[i * n + k] = a[pivot * n + k];
        a[pivot * n + k] = temp;
      }
      det *= -1.0;
    }

    // Check for singular matrix
    if (fabs(a[i * n + i]) < 1e-10) {
      free(a);
      return 0.0;
    }

    // Eliminate column
    for (long j = i + 1; j < n; j++) {
      double factor = a[j * n + i] / a[i * n + i];
      for (long k = i; k < n; k++) {
        a[j * n + k] -= factor * a[i * n + k];
      }
    }

    det *= a[i * n + i];
  }

  free(a);
  return det;
}

// Matrix inverse (simple implementation using Gauss-Jordan)
// Note: For production, use LAPACK dgetri
Matrix *brix_inv(Matrix *m) {
  if (m->rows != m->cols) {
    printf("Error: Inverse requires square matrix\n");
    return NULL;
  }

  long n = m->rows;

  // Create augmented matrix [A | I]
  double *aug = (double *)malloc(n * 2 * n * sizeof(double));

  // Copy original matrix to left side
  for (long i = 0; i < n; i++) {
    for (long j = 0; j < n; j++) {
      aug[i * 2 * n + j] = m->data[i * n + j];
    }
  }

  // Set right side to identity
  for (long i = 0; i < n; i++) {
    for (long j = 0; j < n; j++) {
      aug[i * 2 * n + n + j] = (i == j) ? 1.0 : 0.0;
    }
  }

  // Gauss-Jordan elimination
  for (long i = 0; i < n; i++) {
    // Find pivot
    long pivot = i;
    for (long j = i + 1; j < n; j++) {
      if (fabs(aug[j * 2 * n + i]) > fabs(aug[pivot * 2 * n + i])) {
        pivot = j;
      }
    }

    // Swap rows
    if (pivot != i) {
      for (long k = 0; k < 2 * n; k++) {
        double temp = aug[i * 2 * n + k];
        aug[i * 2 * n + k] = aug[pivot * 2 * n + k];
        aug[pivot * 2 * n + k] = temp;
      }
    }

    // Check for singular matrix
    if (fabs(aug[i * 2 * n + i]) < 1e-10) {
      printf("Error: Matrix is singular (not invertible)\n");
      free(aug);
      return NULL;
    }

    // Scale pivot row
    double pivot_val = aug[i * 2 * n + i];
    for (long k = 0; k < 2 * n; k++) {
      aug[i * 2 * n + k] /= pivot_val;
    }

    // Eliminate column
    for (long j = 0; j < n; j++) {
      if (j != i) {
        double factor = aug[j * 2 * n + i];
        for (long k = 0; k < 2 * n; k++) {
          aug[j * 2 * n + k] -= factor * aug[i * 2 * n + k];
        }
      }
    }
  }

  // Extract inverse from right side
  Matrix *result = matrix_new(n, n);
  for (long i = 0; i < n; i++) {
    for (long j = 0; j < n; j++) {
      result->data[i * n + j] = aug[i * 2 * n + n + j];
    }
  }

  free(aug);
  return result;
}

// ==========================================
// SECTION 7: ZIP FUNCTIONS (v0.9)
// ==========================================

// zip for IntMatrix x IntMatrix → IntMatrix(min_len, 2)
IntMatrix *brix_zip_ii(IntMatrix *arr1, IntMatrix *arr2) {
  // Arrays 1D são armazenados como (1, n), então use cols se rows==1
  long len1 = (arr1->rows == 1) ? arr1->cols : arr1->rows;
  long len2 = (arr2->rows == 1) ? arr2->cols : arr2->rows;
  long min_len = len1 < len2 ? len1 : len2;

  IntMatrix *result = intmatrix_new(min_len, 2);

  for (long i = 0; i < min_len; i++) {
    result->data[i * 2 + 0] = arr1->data[i];  // First element
    result->data[i * 2 + 1] = arr2->data[i];  // Second element
  }

  return result;
}

// zip for IntMatrix x Matrix → Matrix(min_len, 2)
Matrix *brix_zip_if(IntMatrix *arr1, Matrix *arr2) {
  long len1 = (arr1->rows == 1) ? arr1->cols : arr1->rows;
  long len2 = (arr2->rows == 1) ? arr2->cols : arr2->rows;
  long min_len = len1 < len2 ? len1 : len2;

  Matrix *result = matrix_new(min_len, 2);

  for (long i = 0; i < min_len; i++) {
    result->data[i * 2 + 0] = (double)arr1->data[i];  // Convert int to double
    result->data[i * 2 + 1] = arr2->data[i];
  }

  return result;
}

// zip for Matrix x IntMatrix → Matrix(min_len, 2)
Matrix *brix_zip_fi(Matrix *arr1, IntMatrix *arr2) {
  long len1 = (arr1->rows == 1) ? arr1->cols : arr1->rows;
  long len2 = (arr2->rows == 1) ? arr2->cols : arr2->rows;
  long min_len = len1 < len2 ? len1 : len2;

  Matrix *result = matrix_new(min_len, 2);

  for (long i = 0; i < min_len; i++) {
    result->data[i * 2 + 0] = arr1->data[i];
    result->data[i * 2 + 1] = (double)arr2->data[i];  // Convert int to double
  }

  return result;
}

// zip for Matrix x Matrix → Matrix(min_len, 2)
Matrix *brix_zip_ff(Matrix *arr1, Matrix *arr2) {
  long len1 = (arr1->rows == 1) ? arr1->cols : arr1->rows;
  long len2 = (arr2->rows == 1) ? arr2->cols : arr2->rows;
  long min_len = len1 < len2 ? len1 : len2;

  Matrix *result = matrix_new(min_len, 2);

  for (long i = 0; i < min_len; i++) {
    result->data[i * 2 + 0] = arr1->data[i];
    result->data[i * 2 + 1] = arr2->data[i];
  }

  return result;
}