#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <limits.h>
#include <stdint.h>
#include <stdarg.h>
#include <curl/curl.h>
#include <time.h>
#include <errno.h>
#include <setjmp.h>
#include <unistd.h>
#include <sys/wait.h>

// ==========================================
// SECTION -2: MEMORY ALLOCATION (v1.3 - Closures)
// ==========================================

// Heap allocation for closures and future ARC
// These wrappers ensure proper error handling
void* brix_malloc(size_t size) {
    void* ptr = malloc(size);
    if (!ptr && size > 0) {
        fprintf(stderr, "Error: Out of memory (failed to allocate %zu bytes)\n", size);
        exit(1);
    }
    return ptr;
}

void brix_free(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

// ==========================================
// SECTION -1.5: ASYNC/AWAIT SUPPORT (v1.5 Phase 2)
// ==========================================

// Poll result returned by every async poll function.
// status: 0 = PENDING (not yet ready), 1 = READY (value available)
// value:  result as i64 (bit-cast from double for float returns; 0 for void)
typedef struct {
    long long status;
    long long value;
} BrixPollResult;

// Synchronous executor: drives a future to completion on the current thread.
// Keeps calling poll_fn(state_ptr) until it returns READY, then returns the result.
// For Brix v1.5 Phase 2 this is always synchronous (no actual suspension).
BrixPollResult brix_run_to_completion(void* state_ptr,
    BrixPollResult (*poll_fn)(void*))
{
    BrixPollResult result;
    do {
        result = poll_fn(state_ptr);
    } while (result.status == 0);  // 0 = PENDING
    return result;
}

// ==========================================
// SECTION -1: CLOSURES (v1.3)
// ==========================================

// Closure structure for ARC:
// struct { i64 ref_count; void* fn_ptr; void* env_ptr; void (*env_destructor)(void*) }
// env_destructor is NULL when no captured closures need releasing.
typedef struct {
    long ref_count;
    void* fn_ptr;
    void* env_ptr;
    void (*env_destructor)(void* env_ptr);
} BrixClosure;

// Increment reference count (called on copy/assignment)
void* closure_retain(void* closure_ptr) {
    if (!closure_ptr) return NULL;

    BrixClosure* closure = (BrixClosure*)closure_ptr;
    closure->ref_count++;
    return closure_ptr;
}

// Decrement reference count (called when going out of scope)
// Frees memory when ref_count reaches 0
void closure_release(void* closure_ptr) {
    if (!closure_ptr) return;

    BrixClosure* closure = (BrixClosure*)closure_ptr;
    closure->ref_count--;

    if (closure->ref_count == 0) {
        // Free environment (if not null)
        if (closure->env_ptr) {
            // If captured closures live in the env, release them via the destructor
            if (closure->env_destructor) {
                closure->env_destructor(closure->env_ptr);
            }
            brix_free(closure->env_ptr);
        }
        // Note: fn_ptr points to code, not heap data - don't free it
        // Free the closure struct itself
        brix_free(closure_ptr);
    }
}

// ==========================================
// SECTION -1: ATOMS (v1.1 - Elixir-style)
// ==========================================

// Global atom pool for interned strings
// Each atom has a unique ID (index in the pool)
// Atoms are compared by ID (O(1) comparison)

typedef struct {
    char** names;      // Array of atom names
    long count;        // Number of atoms
    long capacity;     // Allocated capacity
} AtomPool;

static AtomPool ATOM_POOL = {NULL, 0, 0};

// Intern an atom (get or create ID)
// Returns unique ID for the atom name
long atom_intern(const char* name) {
    if (name == NULL) {
        fprintf(stderr, "Error: atom_intern called with NULL\n");
        exit(1);
    }

    // Linear search (could optimize with hash table later)
    for (long i = 0; i < ATOM_POOL.count; i++) {
        if (strcmp(ATOM_POOL.names[i], name) == 0) {
            return i;  // Atom already exists
        }
    }

    // Add new atom
    if (ATOM_POOL.count >= ATOM_POOL.capacity) {
        long new_capacity = (ATOM_POOL.capacity == 0) ? 16 : ATOM_POOL.capacity * 2;
        char** new_names = (char**)realloc(ATOM_POOL.names, new_capacity * sizeof(char*));
        if (new_names == NULL) {
            fprintf(stderr, "Error: Failed to allocate atom pool\n");
            exit(1);
        }
        ATOM_POOL.names = new_names;
        ATOM_POOL.capacity = new_capacity;
    }

    // Store atom name (copy string)
    ATOM_POOL.names[ATOM_POOL.count] = strdup(name);
    if (ATOM_POOL.names[ATOM_POOL.count] == NULL) {
        fprintf(stderr, "Error: Failed to allocate atom name\n");
        exit(1);
    }

    return ATOM_POOL.count++;
}

// Get atom name by ID
const char* atom_name(long id) {
    if (id < 0 || id >= ATOM_POOL.count) {
        fprintf(stderr, "Error: Invalid atom ID %ld\n", id);
        exit(1);
    }
    return ATOM_POOL.names[id];
}

// Compare two atoms (by ID)
// Returns 1 if equal, 0 if not equal
int atom_eq(long id1, long id2) {
    return id1 == id2;
}

// Free atom pool memory (cleanup)
void atom_pool_free() {
    for (long i = 0; i < ATOM_POOL.count; i++) {
        free(ATOM_POOL.names[i]);
    }
    free(ATOM_POOL.names);
    ATOM_POOL.names = NULL;
    ATOM_POOL.count = 0;
    ATOM_POOL.capacity = 0;
}

// ==========================================
// SECTION -0.5: RUNTIME ERROR HANDLERS
// ==========================================

// Division by zero error (called from generated code)
void brix_division_by_zero_error() {
    fprintf(stderr, "\n❌ Runtime Error: Division by zero\n");
    exit(1);
}

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
        snprintf(buffer, 100, "%.6g+%.6gim", z.real, z.imag);
    } else {
        snprintf(buffer, 100, "%.6g%.6gim", z.real, z.imag);  // minus sign included in imag
    }
    return buffer;
}

// ==========================================
// SECTION 1: MATRIX (v0.3)
// ==========================================

typedef struct {
  long ref_count;  // ARC reference counting
  long rows;
  long cols;
  double *data;
} Matrix;

Matrix *matrix_new(long rows, long cols) {
  Matrix *m = (Matrix *)malloc(sizeof(Matrix));
  if (!m) {
    fprintf(stderr, "Error: out of memory allocating a %ldx%ld matrix\n", rows, cols);
    exit(1);
  }
  m->ref_count = 1;  // Initialize ARC
  m->rows = rows;
  m->cols = cols;
  m->data = (double *)calloc(rows * cols, sizeof(double));
  // calloc(0, ...) may legitimately return NULL (e.g. a 1x0 result).
  if (!m->data && rows * cols > 0) {
    fprintf(stderr, "Error: out of memory allocating a %ldx%ld matrix\n", rows, cols);
    exit(1);
  }
  return m;
}

// ARC: Increment reference count
void* matrix_retain(Matrix* m) {
    if (!m) return NULL;
    m->ref_count++;
    return m;
}

// ARC: Decrement reference count and free if zero
void matrix_release(Matrix* m) {
    if (!m) return;
    m->ref_count--;

    if (m->ref_count == 0) {
        if (m->data) {
            free(m->data);
        }
        free(m);
    }
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
  long ref_count;  // ARC reference counting
  long rows;
  long cols;
  long *data;  // i64* instead of double*
} IntMatrix;

IntMatrix *intmatrix_new(long rows, long cols) {
  IntMatrix *m = (IntMatrix *)malloc(sizeof(IntMatrix));
  if (!m) {
    fprintf(stderr, "Error: out of memory allocating a %ldx%ld matrix\n", rows, cols);
    exit(1);
  }
  m->ref_count = 1;  // Initialize ARC
  m->rows = rows;
  m->cols = cols;
  m->data = (long *)calloc(rows * cols, sizeof(long));  // calloc zeros memory
  // calloc(0, ...) may legitimately return NULL (e.g. a 1x0 result).
  if (!m->data && rows * cols > 0) {
    fprintf(stderr, "Error: out of memory allocating a %ldx%ld matrix\n", rows, cols);
    exit(1);
  }
  return m;
}

// ARC: Increment reference count
void* intmatrix_retain(IntMatrix* m) {
    if (!m) return NULL;
    m->ref_count++;
    return m;
}

// ARC: Decrement reference count and free if zero
void intmatrix_release(IntMatrix* m) {
    if (!m) return;
    m->ref_count--;

    if (m->ref_count == 0) {
        if (m->data) {
            free(m->data);
        }
        free(m);
    }
}

// Convert IntMatrix to Matrix (automatic promotion for mixed operations)
// Used when IntMatrix operates with Float or Matrix
Matrix *intmatrix_to_matrix(IntMatrix *im) {
  if (im == NULL) {
    fprintf(stderr, "Error: intmatrix_to_matrix called with NULL\n");
    exit(1);
  }

  Matrix *m = matrix_new(im->rows, im->cols);
  long size = im->rows * im->cols;

  // Convert each element from long to double
  for (long i = 0; i < size; i++) {
    m->data[i] = (double)im->data[i];
  }

  return m;
}

// ==========================================
// MATRIX ARITHMETIC OPERATIONS (v1.1)
// ==========================================

// Matrix + scalar
Matrix *matrix_add_scalar(Matrix *m, double scalar) {
  if (m == NULL) {
    fprintf(stderr, "Error: matrix_add_scalar called with NULL\n");
    exit(1);
  }
  Matrix *result = matrix_new(m->rows, m->cols);
  long size = m->rows * m->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = m->data[i] + scalar;
  }
  return result;
}

// Matrix - scalar
Matrix *matrix_sub_scalar(Matrix *m, double scalar) {
  if (m == NULL) {
    fprintf(stderr, "Error: matrix_sub_scalar called with NULL\n");
    exit(1);
  }
  Matrix *result = matrix_new(m->rows, m->cols);
  long size = m->rows * m->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = m->data[i] - scalar;
  }
  return result;
}

// scalar - Matrix
Matrix *scalar_sub_matrix(double scalar, Matrix *m) {
  if (m == NULL) {
    fprintf(stderr, "Error: scalar_sub_matrix called with NULL\n");
    exit(1);
  }
  Matrix *result = matrix_new(m->rows, m->cols);
  long size = m->rows * m->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = scalar - m->data[i];
  }
  return result;
}

// Matrix * scalar
Matrix *matrix_mul_scalar(Matrix *m, double scalar) {
  if (m == NULL) {
    fprintf(stderr, "Error: matrix_mul_scalar called with NULL\n");
    exit(1);
  }
  Matrix *result = matrix_new(m->rows, m->cols);
  long size = m->rows * m->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = m->data[i] * scalar;
  }
  return result;
}

// Matrix / scalar
Matrix *matrix_div_scalar(Matrix *m, double scalar) {
  if (m == NULL) {
    fprintf(stderr, "Error: matrix_div_scalar called with NULL\n");
    exit(1);
  }
  if (scalar == 0.0) {
    fprintf(stderr, "Error: division by zero in matrix_div_scalar\n");
    exit(1);
  }
  Matrix *result = matrix_new(m->rows, m->cols);
  long size = m->rows * m->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = m->data[i] / scalar;
  }
  return result;
}

// scalar / Matrix
Matrix *scalar_div_matrix(double scalar, Matrix *m) {
  if (m == NULL) {
    fprintf(stderr, "Error: scalar_div_matrix called with NULL\n");
    exit(1);
  }
  Matrix *result = matrix_new(m->rows, m->cols);
  long size = m->rows * m->cols;
  for (long i = 0; i < size; i++) {
    if (m->data[i] == 0.0) {
      fprintf(stderr, "Error: division by zero in scalar_div_matrix\n");
      exit(1);
    }
    result->data[i] = scalar / m->data[i];
  }
  return result;
}

// Matrix % scalar (modulo)
Matrix *matrix_mod_scalar(Matrix *m, double scalar) {
  if (m == NULL) {
    fprintf(stderr, "Error: matrix_mod_scalar called with NULL\n");
    exit(1);
  }
  if (scalar == 0.0) {
    fprintf(stderr, "Error: modulo by zero in matrix_mod_scalar\n");
    exit(1);
  }
  Matrix *result = matrix_new(m->rows, m->cols);
  long size = m->rows * m->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = fmod(m->data[i], scalar);
  }
  return result;
}

// Matrix ** scalar (power)
Matrix *matrix_pow_scalar(Matrix *m, double scalar) {
  if (m == NULL) {
    fprintf(stderr, "Error: matrix_pow_scalar called with NULL\n");
    exit(1);
  }
  Matrix *result = matrix_new(m->rows, m->cols);
  long size = m->rows * m->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = pow(m->data[i], scalar);
  }
  return result;
}

// Matrix + Matrix (element-wise)
Matrix *matrix_add_matrix(Matrix *m1, Matrix *m2) {
  if (m1 == NULL || m2 == NULL) {
    fprintf(stderr, "Error: matrix_add_matrix called with NULL\n");
    exit(1);
  }
  if (m1->rows != m2->rows || m1->cols != m2->cols) {
    fprintf(stderr, "Error: matrix dimensions mismatch in addition\n");
    exit(1);
  }
  Matrix *result = matrix_new(m1->rows, m1->cols);
  long size = m1->rows * m1->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = m1->data[i] + m2->data[i];
  }
  return result;
}

// Matrix - Matrix (element-wise)
Matrix *matrix_sub_matrix(Matrix *m1, Matrix *m2) {
  if (m1 == NULL || m2 == NULL) {
    fprintf(stderr, "Error: matrix_sub_matrix called with NULL\n");
    exit(1);
  }
  if (m1->rows != m2->rows || m1->cols != m2->cols) {
    fprintf(stderr, "Error: matrix dimensions mismatch in subtraction\n");
    exit(1);
  }
  Matrix *result = matrix_new(m1->rows, m1->cols);
  long size = m1->rows * m1->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = m1->data[i] - m2->data[i];
  }
  return result;
}

// Matrix * Matrix (element-wise, NOT matrix multiplication)
Matrix *matrix_mul_matrix(Matrix *m1, Matrix *m2) {
  if (m1 == NULL || m2 == NULL) {
    fprintf(stderr, "Error: matrix_mul_matrix called with NULL\n");
    exit(1);
  }
  if (m1->rows != m2->rows || m1->cols != m2->cols) {
    fprintf(stderr, "Error: matrix dimensions mismatch in multiplication\n");
    exit(1);
  }
  Matrix *result = matrix_new(m1->rows, m1->cols);
  long size = m1->rows * m1->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = m1->data[i] * m2->data[i];
  }
  return result;
}

// Matrix / Matrix (element-wise)
Matrix *matrix_div_matrix(Matrix *m1, Matrix *m2) {
  if (m1 == NULL || m2 == NULL) {
    fprintf(stderr, "Error: matrix_div_matrix called with NULL\n");
    exit(1);
  }
  if (m1->rows != m2->rows || m1->cols != m2->cols) {
    fprintf(stderr, "Error: matrix dimensions mismatch in division\n");
    exit(1);
  }
  Matrix *result = matrix_new(m1->rows, m1->cols);
  long size = m1->rows * m1->cols;
  for (long i = 0; i < size; i++) {
    if (m2->data[i] == 0.0) {
      fprintf(stderr, "Error: division by zero in matrix_div_matrix\n");
      exit(1);
    }
    result->data[i] = m1->data[i] / m2->data[i];
  }
  return result;
}

// Matrix % Matrix (element-wise modulo)
Matrix *matrix_mod_matrix(Matrix *m1, Matrix *m2) {
  if (m1 == NULL || m2 == NULL) {
    fprintf(stderr, "Error: matrix_mod_matrix called with NULL\n");
    exit(1);
  }
  if (m1->rows != m2->rows || m1->cols != m2->cols) {
    fprintf(stderr, "Error: matrix dimensions mismatch in modulo\n");
    exit(1);
  }
  Matrix *result = matrix_new(m1->rows, m1->cols);
  long size = m1->rows * m1->cols;
  for (long i = 0; i < size; i++) {
    if (m2->data[i] == 0.0) {
      fprintf(stderr, "Error: modulo by zero in matrix_mod_matrix\n");
      exit(1);
    }
    result->data[i] = fmod(m1->data[i], m2->data[i]);
  }
  return result;
}

// Matrix ** Matrix (element-wise power)
Matrix *matrix_pow_matrix(Matrix *m1, Matrix *m2) {
  if (m1 == NULL || m2 == NULL) {
    fprintf(stderr, "Error: matrix_pow_matrix called with NULL\n");
    exit(1);
  }
  if (m1->rows != m2->rows || m1->cols != m2->cols) {
    fprintf(stderr, "Error: matrix dimensions mismatch in power\n");
    exit(1);
  }
  Matrix *result = matrix_new(m1->rows, m1->cols);
  long size = m1->rows * m1->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = pow(m1->data[i], m2->data[i]);
  }
  return result;
}

// ==========================================
// INTMATRIX ARITHMETIC OPERATIONS (v1.1)
// ==========================================

// IntMatrix + Int (scalar)
IntMatrix *intmatrix_add_scalar(IntMatrix *m, long scalar) {
  if (m == NULL) {
    fprintf(stderr, "Error: intmatrix_add_scalar called with NULL\n");
    exit(1);
  }
  IntMatrix *result = intmatrix_new(m->rows, m->cols);
  long size = m->rows * m->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = m->data[i] + scalar;
  }
  return result;
}

// IntMatrix - Int
IntMatrix *intmatrix_sub_scalar(IntMatrix *m, long scalar) {
  if (m == NULL) {
    fprintf(stderr, "Error: intmatrix_sub_scalar called with NULL\n");
    exit(1);
  }
  IntMatrix *result = intmatrix_new(m->rows, m->cols);
  long size = m->rows * m->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = m->data[i] - scalar;
  }
  return result;
}

// Int - IntMatrix
IntMatrix *scalar_sub_intmatrix(long scalar, IntMatrix *m) {
  if (m == NULL) {
    fprintf(stderr, "Error: scalar_sub_intmatrix called with NULL\n");
    exit(1);
  }
  IntMatrix *result = intmatrix_new(m->rows, m->cols);
  long size = m->rows * m->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = scalar - m->data[i];
  }
  return result;
}

// IntMatrix * Int
IntMatrix *intmatrix_mul_scalar(IntMatrix *m, long scalar) {
  if (m == NULL) {
    fprintf(stderr, "Error: intmatrix_mul_scalar called with NULL\n");
    exit(1);
  }
  IntMatrix *result = intmatrix_new(m->rows, m->cols);
  long size = m->rows * m->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = m->data[i] * scalar;
  }
  return result;
}

// IntMatrix / Int
IntMatrix *intmatrix_div_scalar(IntMatrix *m, long scalar) {
  if (m == NULL) {
    fprintf(stderr, "Error: intmatrix_div_scalar called with NULL\n");
    exit(1);
  }
  if (scalar == 0) {
    fprintf(stderr, "Error: division by zero in intmatrix_div_scalar\n");
    exit(1);
  }
  IntMatrix *result = intmatrix_new(m->rows, m->cols);
  long size = m->rows * m->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = m->data[i] / scalar;  // Integer division
  }
  return result;
}

// IntMatrix % Int
IntMatrix *intmatrix_mod_scalar(IntMatrix *m, long scalar) {
  if (m == NULL) {
    fprintf(stderr, "Error: intmatrix_mod_scalar called with NULL\n");
    exit(1);
  }
  if (scalar == 0) {
    fprintf(stderr, "Error: modulo by zero in intmatrix_mod_scalar\n");
    exit(1);
  }
  IntMatrix *result = intmatrix_new(m->rows, m->cols);
  long size = m->rows * m->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = m->data[i] % scalar;
  }
  return result;
}

// IntMatrix ** Int (power with integer result)
IntMatrix *intmatrix_pow_scalar(IntMatrix *m, long scalar) {
  if (m == NULL) {
    fprintf(stderr, "Error: intmatrix_pow_scalar called with NULL\n");
    exit(1);
  }
  IntMatrix *result = intmatrix_new(m->rows, m->cols);
  long size = m->rows * m->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = (long)pow((double)m->data[i], (double)scalar);
  }
  return result;
}

// IntMatrix + IntMatrix (element-wise)
IntMatrix *intmatrix_add_intmatrix(IntMatrix *m1, IntMatrix *m2) {
  if (m1 == NULL || m2 == NULL) {
    fprintf(stderr, "Error: intmatrix_add_intmatrix called with NULL\n");
    exit(1);
  }
  if (m1->rows != m2->rows || m1->cols != m2->cols) {
    fprintf(stderr, "Error: intmatrix dimensions mismatch in addition\n");
    exit(1);
  }
  IntMatrix *result = intmatrix_new(m1->rows, m1->cols);
  long size = m1->rows * m1->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = m1->data[i] + m2->data[i];
  }
  return result;
}

// IntMatrix - IntMatrix (element-wise)
IntMatrix *intmatrix_sub_intmatrix(IntMatrix *m1, IntMatrix *m2) {
  if (m1 == NULL || m2 == NULL) {
    fprintf(stderr, "Error: intmatrix_sub_intmatrix called with NULL\n");
    exit(1);
  }
  if (m1->rows != m2->rows || m1->cols != m2->cols) {
    fprintf(stderr, "Error: intmatrix dimensions mismatch in subtraction\n");
    exit(1);
  }
  IntMatrix *result = intmatrix_new(m1->rows, m1->cols);
  long size = m1->rows * m1->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = m1->data[i] - m2->data[i];
  }
  return result;
}

// IntMatrix * IntMatrix (element-wise)
IntMatrix *intmatrix_mul_intmatrix(IntMatrix *m1, IntMatrix *m2) {
  if (m1 == NULL || m2 == NULL) {
    fprintf(stderr, "Error: intmatrix_mul_intmatrix called with NULL\n");
    exit(1);
  }
  if (m1->rows != m2->rows || m1->cols != m2->cols) {
    fprintf(stderr, "Error: intmatrix dimensions mismatch in multiplication\n");
    exit(1);
  }
  IntMatrix *result = intmatrix_new(m1->rows, m1->cols);
  long size = m1->rows * m1->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = m1->data[i] * m2->data[i];
  }
  return result;
}

// IntMatrix / IntMatrix (element-wise integer division)
IntMatrix *intmatrix_div_intmatrix(IntMatrix *m1, IntMatrix *m2) {
  if (m1 == NULL || m2 == NULL) {
    fprintf(stderr, "Error: intmatrix_div_intmatrix called with NULL\n");
    exit(1);
  }
  if (m1->rows != m2->rows || m1->cols != m2->cols) {
    fprintf(stderr, "Error: intmatrix dimensions mismatch in division\n");
    exit(1);
  }
  IntMatrix *result = intmatrix_new(m1->rows, m1->cols);
  long size = m1->rows * m1->cols;
  for (long i = 0; i < size; i++) {
    if (m2->data[i] == 0) {
      fprintf(stderr, "Error: division by zero in intmatrix_div_intmatrix\n");
      exit(1);
    }
    result->data[i] = m1->data[i] / m2->data[i];
  }
  return result;
}

// IntMatrix % IntMatrix (element-wise modulo)
IntMatrix *intmatrix_mod_intmatrix(IntMatrix *m1, IntMatrix *m2) {
  if (m1 == NULL || m2 == NULL) {
    fprintf(stderr, "Error: intmatrix_mod_intmatrix called with NULL\n");
    exit(1);
  }
  if (m1->rows != m2->rows || m1->cols != m2->cols) {
    fprintf(stderr, "Error: intmatrix dimensions mismatch in modulo\n");
    exit(1);
  }
  IntMatrix *result = intmatrix_new(m1->rows, m1->cols);
  long size = m1->rows * m1->cols;
  for (long i = 0; i < size; i++) {
    if (m2->data[i] == 0) {
      fprintf(stderr, "Error: modulo by zero in intmatrix_mod_intmatrix\n");
      exit(1);
    }
    result->data[i] = m1->data[i] % m2->data[i];
  }
  return result;
}

// IntMatrix ** IntMatrix (element-wise power)
IntMatrix *intmatrix_pow_intmatrix(IntMatrix *m1, IntMatrix *m2) {
  if (m1 == NULL || m2 == NULL) {
    fprintf(stderr, "Error: intmatrix_pow_intmatrix called with NULL\n");
    exit(1);
  }
  if (m1->rows != m2->rows || m1->cols != m2->cols) {
    fprintf(stderr, "Error: intmatrix dimensions mismatch in power\n");
    exit(1);
  }
  IntMatrix *result = intmatrix_new(m1->rows, m1->cols);
  long size = m1->rows * m1->cols;
  for (long i = 0; i < size; i++) {
    result->data[i] = (long)pow((double)m1->data[i], (double)m2->data[i]);
  }
  return result;
}

// ==========================================
// SECTION 1.6: COMPLEXMATRIX (v1.0)
// ==========================================

typedef struct {
  long ref_count;  // ARC reference counting
  long rows;
  long cols;
  Complex *data;  // Array of Complex structs
} ComplexMatrix;

ComplexMatrix *complexmatrix_new(long rows, long cols) {
  ComplexMatrix *m = (ComplexMatrix *)malloc(sizeof(ComplexMatrix));
  m->ref_count = 1;  // Initialize ARC
  m->rows = rows;
  m->cols = cols;
  m->data = (Complex *)calloc(rows * cols, sizeof(Complex));  // calloc zeros memory
  return m;
}

// ARC: Increment reference count
void* complexmatrix_retain(ComplexMatrix* m) {
    if (!m) return NULL;
    m->ref_count++;
    return m;
}

// ARC: Decrement reference count and free if zero
void complexmatrix_release(ComplexMatrix* m) {
    if (!m) return;
    m->ref_count--;

    if (m->ref_count == 0) {
        if (m->data) {
            free(m->data);
        }
        free(m);
    }
}

// ==========================================
// SECTION 1.7: LINEAR ALGEBRA - LAPACK (v1.0)
// ==========================================

// Helper: Convert Matrix to column-major format for LAPACK
void matrix_to_colmajor(Matrix *m, double *output) {
  for (long j = 0; j < m->cols; j++) {
    for (long i = 0; i < m->rows; i++) {
      output[j * m->rows + i] = m->data[i * m->cols + j];
    }
  }
}

// LAPACK eigenvalue computation wrapper
// Returns ComplexMatrix with shape (n, 1) containing eigenvalues
ComplexMatrix *brix_eigvals(Matrix *A) {
  if (A->rows != A->cols) {
    fprintf(stderr, "Error: eigvals() requires square matrix\n");
    exit(1);
  }

  long n = A->rows;

  // Convert to column-major format (LAPACK requirement)
  double *a = (double *)malloc(n * n * sizeof(double));
  matrix_to_colmajor(A, a);

  // Allocate output arrays
  double *wr = (double *)malloc(n * sizeof(double));  // Real parts
  double *wi = (double *)malloc(n * sizeof(double));  // Imaginary parts

  // Dummy arrays for eigenvectors (not computed)
  double vl_dummy = 0;
  double vr_dummy = 0;

  // Call LAPACK dgeev
  // Parameters: jobvl, jobvr, n, a, lda, wr, wi, vl, ldvl, vr, ldvr
  int info;
  char jobvl = 'N';  // Don't compute left eigenvectors
  char jobvr = 'N';  // Don't compute right eigenvectors
  int n_int = (int)n;

  // External LAPACK declaration
  extern void dgeev_(char *jobvl, char *jobvr, int *n, double *a, int *lda,
                     double *wr, double *wi, double *vl, int *ldvl,
                     double *vr, int *ldvr, double *work, int *lwork, int *info);

  // Query optimal work array size
  double work_query;
  int lwork = -1;
  dgeev_(&jobvl, &jobvr, &n_int, a, &n_int, wr, wi, &vl_dummy, &n_int,
         &vr_dummy, &n_int, &work_query, &lwork, &info);

  lwork = (int)work_query;
  double *work = (double *)malloc(lwork * sizeof(double));

  // Compute eigenvalues
  dgeev_(&jobvl, &jobvr, &n_int, a, &n_int, wr, wi, &vl_dummy, &n_int,
         &vr_dummy, &n_int, work, &lwork, &info);

  if (info != 0) {
    fprintf(stderr, "Error: LAPACK dgeev failed with info=%d\n", info);
    exit(1);
  }

  // Create ComplexMatrix result (n x 1)
  ComplexMatrix *result = complexmatrix_new(n, 1);
  for (long i = 0; i < n; i++) {
    result->data[i].real = wr[i];
    result->data[i].imag = wi[i];
  }

  // Cleanup
  free(a);
  free(wr);
  free(wi);
  free(work);

  return result;
}

// LAPACK eigenvector computation wrapper
// Returns ComplexMatrix with shape (n, n) containing eigenvectors as columns
ComplexMatrix *brix_eigvecs(Matrix *A) {
  if (A->rows != A->cols) {
    fprintf(stderr, "Error: eigvecs() requires square matrix\n");
    exit(1);
  }

  long n = A->rows;

  // Convert to column-major format
  double *a = (double *)malloc(n * n * sizeof(double));
  matrix_to_colmajor(A, a);

  // Allocate output arrays
  double *wr = (double *)malloc(n * sizeof(double));  // Real parts of eigenvalues
  double *wi = (double *)malloc(n * sizeof(double));  // Imaginary parts of eigenvalues
  double *vr = (double *)malloc(n * n * sizeof(double));  // Right eigenvectors

  // Dummy for left eigenvectors
  double vl_dummy = 0;

  // Call LAPACK dgeev
  int info;
  char jobvl = 'N';  // Don't compute left eigenvectors
  char jobvr = 'V';  // Compute right eigenvectors
  int n_int = (int)n;

  extern void dgeev_(char *jobvl, char *jobvr, int *n, double *a, int *lda,
                     double *wr, double *wi, double *vl, int *ldvl,
                     double *vr, int *ldvr, double *work, int *lwork, int *info);

  // Query optimal work array size
  double work_query;
  int lwork = -1;
  dgeev_(&jobvl, &jobvr, &n_int, a, &n_int, wr, wi, &vl_dummy, &n_int,
         vr, &n_int, &work_query, &lwork, &info);

  lwork = (int)work_query;
  double *work = (double *)malloc(lwork * sizeof(double));

  // Compute eigenvectors
  dgeev_(&jobvl, &jobvr, &n_int, a, &n_int, wr, wi, &vl_dummy, &n_int,
         vr, &n_int, work, &lwork, &info);

  if (info != 0) {
    fprintf(stderr, "Error: LAPACK dgeev failed with info=%d\n", info);
    exit(1);
  }

  // Create ComplexMatrix result (n x n)
  // LAPACK stores eigenvectors in columns
  ComplexMatrix *result = complexmatrix_new(n, n);

  long col = 0;
  while (col < n) {
    if (wi[col] == 0.0) {
      // Real eigenvalue - eigenvector is real
      for (long row = 0; row < n; row++) {
        result->data[row * n + col].real = vr[col * n + row];
        result->data[row * n + col].imag = 0.0;
      }
      col++;
    } else {
      // Complex conjugate pair of eigenvalues
      // LAPACK stores: v[col] = real part, v[col+1] = imaginary part
      for (long row = 0; row < n; row++) {
        // First eigenvector: real + i*imag
        result->data[row * n + col].real = vr[col * n + row];
        result->data[row * n + col].imag = vr[(col + 1) * n + row];

        // Second eigenvector: real - i*imag (complex conjugate)
        result->data[row * n + (col + 1)].real = vr[col * n + row];
        result->data[row * n + (col + 1)].imag = -vr[(col + 1) * n + row];
      }
      col += 2;
    }
  }

  // Cleanup
  free(a);
  free(wr);
  free(wi);
  free(vr);
  free(work);

  return result;
}

// ------------------------------------------------------------------
// LU decomposition (v1.8 Grupo B): A = P * L * U  (square A, LAPACK dgetrf)
//
// Returns a heap LUResult whose layout is a plain triple of pointers so the
// codegen can load it and pack a BrixType::Tuple(Matrix, Matrix, IntMatrix)
// directly. The container is freed by the codegen after the fields are
// extracted; the three matrices carry ref_count = 1 (owned by whoever binds
// them, e.g. `var { L, U, P } := math.lu(A)`).
//
//   L : n x n unit lower-triangular (1s on the diagonal)
//   U : n x n upper-triangular
//   P : n x 1 IntMatrix, a 0-based row-permutation vector such that
//       row i of (L*U) equals row P[i] of A  (i.e. P applied to A gives L*U).
// ------------------------------------------------------------------
typedef struct {
  Matrix *L;
  Matrix *U;
  IntMatrix *P;
} LUResult;

LUResult *math_lu(Matrix *A) {
  if (A->rows != A->cols) {
    fprintf(stderr, "Error: lu() requires a square matrix\n");
    exit(1);
  }

  long n = A->rows;

  // Column-major copy for LAPACK (factored in place).
  double *a = (double *)malloc(n * n * sizeof(double));
  matrix_to_colmajor(A, a);

  int *ipiv = (int *)malloc(n * sizeof(int));
  int n_int = (int)n;
  int info;

  extern void dgetrf_(int *m, int *n, double *a, int *lda, int *ipiv,
                      int *info);
  dgetrf_(&n_int, &n_int, a, &n_int, ipiv, &info);

  // info < 0: illegal argument (fatal). info > 0: U(i,i) is exactly zero
  // (A is singular), but the factorization is still complete and usable, so
  // we return L/U/P as usual — it is math.solve() that rejects singular
  // systems, not math.lu().
  if (info < 0) {
    fprintf(stderr, "Error: LAPACK dgetrf illegal argument (info=%d)\n", info);
    exit(1);
  }

  // Split the in-place factors into L (unit lower) and U (upper).
  // Column-major element (i, j) lives at a[j * n + i].
  Matrix *L = matrix_new(n, n);
  Matrix *U = matrix_new(n, n);
  for (long i = 0; i < n; i++) {
    for (long j = 0; j < n; j++) {
      double v = a[j * n + i];
      if (i > j) {
        L->data[i * n + j] = v;   // strictly lower
        U->data[i * n + j] = 0.0;
      } else if (i == j) {
        L->data[i * n + j] = 1.0; // unit diagonal
        U->data[i * n + j] = v;
      } else {
        L->data[i * n + j] = 0.0;
        U->data[i * n + j] = v;   // strictly upper
      }
    }
  }

  // Convert LAPACK ipiv (1-based successive row swaps) into a 0-based
  // permutation vector: start from the identity and apply each swap in order.
  IntMatrix *P = intmatrix_new(n, 1);
  for (long i = 0; i < n; i++) {
    P->data[i] = i;
  }
  for (long k = 0; k < n; k++) {
    long target = (long)ipiv[k] - 1; // to 0-based
    if (target != k) {
      long tmp = P->data[k];
      P->data[k] = P->data[target];
      P->data[target] = tmp;
    }
  }

  free(a);
  free(ipiv);

  LUResult *res = (LUResult *)malloc(sizeof(LUResult));
  res->L = L;
  res->U = U;
  res->P = P;
  return res;
}

// ------------------------------------------------------------------
// QR decomposition (v1.8 Grupo B): A = Q * R  (full, dgeqrf + dorgqr)
//   Q : m x m orthogonal
//   R : m x n upper-triangular
// Same container convention as LUResult (a plain struct of Matrix pointers).
// ------------------------------------------------------------------
typedef struct {
  Matrix *Q;
  Matrix *R;
} QRResult;

QRResult *math_qr(Matrix *A) {
  long m = A->rows;
  long n = A->cols;
  if (m <= 0 || n <= 0) {
    fprintf(stderr, "Error: qr() requires a non-empty matrix\n");
    exit(1);
  }
  long k = (m < n) ? m : n; // number of Householder reflectors

  int m_int = (int)m, n_int = (int)n, k_int = (int)k;
  int info;

  extern void dgeqrf_(int *m, int *n, double *a, int *lda, double *tau,
                      double *work, int *lwork, int *info);
  extern void dorgqr_(int *m, int *n, int *k, double *a, int *lda, double *tau,
                      double *work, int *lwork, int *info);

  // Column-major copy of A (m x n).
  double *a = (double *)malloc(m * n * sizeof(double));
  matrix_to_colmajor(A, a);
  double *tau = (double *)malloc((k > 0 ? k : 1) * sizeof(double));

  // Factorize: R + reflectors stored in `a`.
  double wq;
  int lwork = -1;
  dgeqrf_(&m_int, &n_int, a, &m_int, tau, &wq, &lwork, &info);
  lwork = (int)wq;
  double *work = (double *)malloc((lwork > 0 ? lwork : 1) * sizeof(double));
  dgeqrf_(&m_int, &n_int, a, &m_int, tau, work, &lwork, &info);
  if (info < 0) {
    fprintf(stderr, "Error: LAPACK dgeqrf illegal argument (info=%d)\n", info);
    exit(1);
  }

  // R (m x n, upper-triangular) — extract before dorgqr overwrites `a`.
  Matrix *R = matrix_new(m, n);
  for (long i = 0; i < m; i++) {
    for (long j = 0; j < n; j++) {
      R->data[i * n + j] = (i <= j) ? a[j * m + i] : 0.0;
    }
  }

  // Full Q (m x m): copy the k reflector columns into an m x m buffer and let
  // dorgqr build the m orthonormal columns.
  double *q = (double *)malloc(m * m * sizeof(double));
  for (long j = 0; j < m; j++) {
    for (long i = 0; i < m; i++) {
      q[j * m + i] = (j < n) ? a[j * m + i] : 0.0;
    }
  }
  free(work);
  lwork = -1;
  dorgqr_(&m_int, &m_int, &k_int, q, &m_int, tau, &wq, &lwork, &info);
  lwork = (int)wq;
  work = (double *)malloc((lwork > 0 ? lwork : 1) * sizeof(double));
  dorgqr_(&m_int, &m_int, &k_int, q, &m_int, tau, work, &lwork, &info);
  if (info < 0) {
    fprintf(stderr, "Error: LAPACK dorgqr illegal argument (info=%d)\n", info);
    exit(1);
  }

  Matrix *Q = matrix_new(m, m);
  for (long i = 0; i < m; i++) {
    for (long j = 0; j < m; j++) {
      Q->data[i * m + j] = q[j * m + i];
    }
  }

  free(a);
  free(tau);
  free(work);
  free(q);

  QRResult *res = (QRResult *)malloc(sizeof(QRResult));
  res->Q = Q;
  res->R = R;
  return res;
}

// ------------------------------------------------------------------
// SVD (v1.8 Grupo B): A = U * diag(S) * Vt  (full, LAPACK dgesdd)
//   U  : m x m orthogonal
//   S  : min(m,n) x 1 column vector of singular values (descending)
//   Vt : n x n orthogonal (V transposed)
// ------------------------------------------------------------------
typedef struct {
  Matrix *U;
  Matrix *S;
  Matrix *Vt;
} SVDResult;

SVDResult *math_svd(Matrix *A) {
  long m = A->rows;
  long n = A->cols;
  if (m <= 0 || n <= 0) {
    fprintf(stderr, "Error: svd() requires a non-empty matrix\n");
    exit(1);
  }
  long k = (m < n) ? m : n;

  int m_int = (int)m, n_int = (int)n, info;

  extern void dgesdd_(char *jobz, int *m, int *n, double *a, int *lda,
                      double *s, double *u, int *ldu, double *vt, int *ldvt,
                      double *work, int *lwork, int *iwork, int *info);

  double *a = (double *)malloc(m * n * sizeof(double));
  matrix_to_colmajor(A, a);
  double *s = (double *)malloc((k > 0 ? k : 1) * sizeof(double));
  double *u = (double *)malloc(m * m * sizeof(double));
  double *vt = (double *)malloc(n * n * sizeof(double));
  int *iwork = (int *)malloc(8 * (k > 0 ? k : 1) * sizeof(int));
  char jobz = 'A'; // full U (m x m) and Vt (n x n)

  double wq;
  int lwork = -1;
  dgesdd_(&jobz, &m_int, &n_int, a, &m_int, s, u, &m_int, vt, &n_int, &wq,
          &lwork, iwork, &info);
  lwork = (int)wq;
  double *work = (double *)malloc((lwork > 0 ? lwork : 1) * sizeof(double));
  dgesdd_(&jobz, &m_int, &n_int, a, &m_int, s, u, &m_int, vt, &n_int, work,
          &lwork, iwork, &info);
  // info < 0: illegal argument; info > 0: DBDSDC did not converge. Both are
  // genuine failures for SVD (unlike LU, there is no useful partial result).
  if (info != 0) {
    fprintf(stderr, "Error: LAPACK dgesdd failed with info=%d\n", info);
    exit(1);
  }

  Matrix *U = matrix_new(m, m);
  for (long i = 0; i < m; i++) {
    for (long j = 0; j < m; j++) {
      U->data[i * m + j] = u[j * m + i];
    }
  }
  Matrix *S = matrix_new(k, 1);
  for (long i = 0; i < k; i++) {
    S->data[i] = s[i];
  }
  Matrix *Vt = matrix_new(n, n);
  for (long i = 0; i < n; i++) {
    for (long j = 0; j < n; j++) {
      Vt->data[i * n + j] = vt[j * n + i];
    }
  }

  free(a);
  free(s);
  free(u);
  free(vt);
  free(iwork);
  free(work);

  SVDResult *res = (SVDResult *)malloc(sizeof(SVDResult));
  res->U = U;
  res->S = S;
  res->Vt = Vt;
  return res;
}

// ------------------------------------------------------------------
// Cholesky (v1.8 Grupo B): A = L * L^T for symmetric positive-definite A.
// dpotrf('L'); the upper triangle of the working copy keeps the original
// input, so we zero it explicitly when extracting the lower-triangular L.
// ------------------------------------------------------------------
Matrix *math_cholesky(Matrix *A) {
  if (A->rows <= 0 || A->cols <= 0) {
    fprintf(stderr, "Error: cholesky() requires a non-empty matrix\n");
    exit(1);
  }
  if (A->rows != A->cols) {
    fprintf(stderr, "Error: cholesky() requires a square matrix\n");
    exit(1);
  }
  long n = A->rows;
  double *a = (double *)malloc(n * n * sizeof(double));
  matrix_to_colmajor(A, a);

  char uplo = 'L';
  int n_int = (int)n, info;
  extern void dpotrf_(char *uplo, int *n, double *a, int *lda, int *info);
  dpotrf_(&uplo, &n_int, a, &n_int, &info);
  if (info != 0) {
    fprintf(stderr,
            "Error: cholesky() failed (matrix not positive definite or "
            "illegal argument, info=%d)\n",
            info);
    exit(1);
  }

  // Lower-triangular L in the lower triangle of column-major `a`; zero the
  // strict upper triangle (dpotrf leaves the original values there).
  Matrix *L = matrix_new(n, n);
  for (long i = 0; i < n; i++) {
    for (long j = 0; j < n; j++) {
      L->data[i * n + j] = (i >= j) ? a[j * n + i] : 0.0;
    }
  }
  free(a);
  return L;
}

// ------------------------------------------------------------------
// Solve (v1.8 Grupo B): x such that A*x = b, via LAPACK dgesv (LU + solve).
// A is n x n. b may be an n x nrhs matrix, or a length-n vector in either
// orientation (Brix 1-D literals are 1 x n); x is returned in b's shape.
// A singular A makes dgesv report info > 0 — solve rejects it (fatal), unlike
// math.lu which returns the factors regardless.
// ------------------------------------------------------------------
Matrix *math_solve(Matrix *A, Matrix *b) {
  if (A->rows <= 0 || A->cols <= 0) {
    fprintf(stderr, "Error: solve() requires a non-empty coefficient matrix\n");
    exit(1);
  }
  if (A->rows != A->cols) {
    fprintf(stderr, "Error: solve() requires a square coefficient matrix\n");
    exit(1);
  }
  long n = A->rows;

  long nrhs;
  int b_is_row = 0;
  if (b->rows == n) {
    nrhs = b->cols; // n x nrhs (includes n x 1 column vector)
  } else if (b->rows == 1 && b->cols == n) {
    nrhs = 1;
    b_is_row = 1; // 1 x n row vector → single RHS
  } else {
    fprintf(stderr, "Error: solve() RHS dimension mismatch\n");
    exit(1);
  }

  double *a = (double *)malloc(n * n * sizeof(double));
  matrix_to_colmajor(A, a);
  double *bmat = (double *)malloc(n * nrhs * sizeof(double));
  if (b_is_row) {
    for (long i = 0; i < n; i++)
      bmat[i] = b->data[i]; // 1 x n → n x 1 (contiguous)
  } else {
    matrix_to_colmajor(b, bmat);
  }

  int *ipiv = (int *)malloc(n * sizeof(int));
  int n_int = (int)n, nrhs_int = (int)nrhs, info;
  extern void dgesv_(int *n, int *nrhs, double *a, int *lda, int *ipiv,
                     double *b, int *ldb, int *info);
  dgesv_(&n_int, &nrhs_int, a, &n_int, ipiv, bmat, &n_int, &info);
  if (info != 0) {
    fprintf(stderr,
            "Error: solve() failed (singular matrix or illegal argument, "
            "info=%d)\n",
            info);
    exit(1);
  }

  Matrix *x;
  if (b_is_row) {
    x = matrix_new(1, n);
    for (long i = 0; i < n; i++)
      x->data[i] = bmat[i]; // n x 1 → 1 x n
  } else {
    x = matrix_new(n, nrhs);
    for (long i = 0; i < n; i++)
      for (long j = 0; j < nrhs; j++)
        x->data[i * nrhs + j] = bmat[j * n + i];
  }

  free(a);
  free(bmat);
  free(ipiv);
  return x;
}

// ------------------------------------------------------------------
// Norms (v1.8 Grupo B).
//   math_norm_vec: L2 (Euclidean) norm of a vector (any shape, flattened).
//   math_norm_mat: 0 = Frobenius, 1 = 1-norm (max column sum),
//                  2 = inf-norm (max row sum).
// ------------------------------------------------------------------
double math_norm_vec(Matrix *v) {
  long total = v->rows * v->cols;
  double sum = 0.0;
  for (long i = 0; i < total; i++)
    sum += v->data[i] * v->data[i];
  return sqrt(sum);
}

double math_norm_mat(Matrix *A, long norm_type) {
  if (norm_type < 0 || norm_type > 2) {
    fprintf(stderr,
            "Error: norm_mat() type must be 0 (Frobenius), 1 (1-norm) or "
            "2 (inf-norm), got %ld\n",
            norm_type);
    exit(1);
  }
  long m = A->rows, n = A->cols;
  if (norm_type == 1) {
    double maxcol = 0.0;
    for (long j = 0; j < n; j++) {
      double col = 0.0;
      for (long i = 0; i < m; i++)
        col += fabs(A->data[i * n + j]);
      if (col > maxcol)
        maxcol = col;
    }
    return maxcol;
  } else if (norm_type == 2) {
    double maxrow = 0.0;
    for (long i = 0; i < m; i++) {
      double row = 0.0;
      for (long j = 0; j < n; j++)
        row += fabs(A->data[i * n + j]);
      if (row > maxrow)
        maxrow = row;
    }
    return maxrow;
  }
  // Frobenius (default / norm_type == 0)
  double sum = 0.0;
  for (long i = 0; i < m * n; i++)
    sum += A->data[i] * A->data[i];
  return sqrt(sum);
}

// ==========================================
// SECTION 1.8: ARRAY METHODS (v1.7 Grupo B)
// ==========================================

// Forward declaration: compare_doubles is defined later, in SECTION 1
// (ERROR HANDLING / stats), reused here for ascending sort.
static int compare_doubles(const void *a, const void *b);

// Comparison function for qsort (long/i64)
static int compare_longs(const void *a, const void *b) {
  long la = *(const long *)a;
  long lb = *(const long *)b;
  return (la > lb) - (la < lb);
}

// Comparison function for qsort (descending doubles)
static int compare_doubles_desc(const void *a, const void *b) {
  double da = *(const double *)a;
  double db = *(const double *)b;
  return (db > da) - (db < da);
}

// Comparison function for qsort (descending longs)
static int compare_longs_desc(const void *a, const void *b) {
  long la = *(const long *)a;
  long lb = *(const long *)b;
  return (lb > la) - (lb < la);
}

// --- Sort ---

Matrix* matrix_sort_asc(Matrix* m) {
  long total = m->rows * m->cols;
  Matrix *result = matrix_new(m->rows, m->cols);
  if (total == 0) return result;
  memcpy(result->data, m->data, total * sizeof(double));
  qsort(result->data, total, sizeof(double), compare_doubles);
  return result;
}

Matrix* matrix_sort_desc(Matrix* m) {
  long total = m->rows * m->cols;
  Matrix *result = matrix_new(m->rows, m->cols);
  if (total == 0) return result;
  memcpy(result->data, m->data, total * sizeof(double));
  qsort(result->data, total, sizeof(double), compare_doubles_desc);
  return result;
}

IntMatrix* intmatrix_sort_asc(IntMatrix* m) {
  long total = m->rows * m->cols;
  IntMatrix *result = intmatrix_new(m->rows, m->cols);
  if (total == 0) return result;
  memcpy(result->data, m->data, total * sizeof(long));
  qsort(result->data, total, sizeof(long), compare_longs);
  return result;
}

IntMatrix* intmatrix_sort_desc(IntMatrix* m) {
  long total = m->rows * m->cols;
  IntMatrix *result = intmatrix_new(m->rows, m->cols);
  if (total == 0) return result;
  memcpy(result->data, m->data, total * sizeof(long));
  qsort(result->data, total, sizeof(long), compare_longs_desc);
  return result;
}

// --- Min/Max ---

double brix_matrix_min(Matrix* m) {
  long total = m->rows * m->cols;
  if (total == 0) return 0.0;
  double min_val = m->data[0];
  for (long i = 1; i < total; i++) {
    if (m->data[i] < min_val) min_val = m->data[i];
  }
  return min_val;
}

double brix_matrix_max(Matrix* m) {
  long total = m->rows * m->cols;
  if (total == 0) return 0.0;
  double max_val = m->data[0];
  for (long i = 1; i < total; i++) {
    if (m->data[i] > max_val) max_val = m->data[i];
  }
  return max_val;
}

long brix_intmatrix_min(IntMatrix* m) {
  long total = m->rows * m->cols;
  if (total == 0) return 0;
  long min_val = m->data[0];
  for (long i = 1; i < total; i++) {
    if (m->data[i] < min_val) min_val = m->data[i];
  }
  return min_val;
}

long brix_intmatrix_max(IntMatrix* m) {
  long total = m->rows * m->cols;
  if (total == 0) return 0;
  long max_val = m->data[0];
  for (long i = 1; i < total; i++) {
    if (m->data[i] > max_val) max_val = m->data[i];
  }
  return max_val;
}

// --- Flatten ---

Matrix* matrix_flatten(Matrix* m) {
  long total = m->rows * m->cols;
  Matrix *result = matrix_new(1, total);
  if (total > 0) memcpy(result->data, m->data, total * sizeof(double));
  return result;
}

IntMatrix* intmatrix_flatten(IntMatrix* m) {
  long total = m->rows * m->cols;
  IntMatrix *result = intmatrix_new(1, total);
  if (total > 0) memcpy(result->data, m->data, total * sizeof(long));
  return result;
}

// --- Unique (preserves order of first appearance, O(n^2)) ---

Matrix* matrix_unique(Matrix* m) {
  long total = m->rows * m->cols;
  double *temp = (double *)brix_malloc(total > 0 ? total * sizeof(double) : sizeof(double));
  long count = 0;

  for (long i = 0; i < total; i++) {
    double val = m->data[i];
    int found = 0;
    for (long j = 0; j < count; j++) {
      if (temp[j] == val) { found = 1; break; }
    }
    if (!found) {
      temp[count] = val;
      count++;
    }
  }

  Matrix *result = matrix_new(1, count);
  if (count > 0) memcpy(result->data, temp, count * sizeof(double));
  brix_free(temp);
  return result;
}

IntMatrix* intmatrix_unique(IntMatrix* m) {
  long total = m->rows * m->cols;
  long *temp = (long *)brix_malloc(total > 0 ? total * sizeof(long) : sizeof(long));
  long count = 0;

  for (long i = 0; i < total; i++) {
    long val = m->data[i];
    int found = 0;
    for (long j = 0; j < count; j++) {
      if (temp[j] == val) { found = 1; break; }
    }
    if (!found) {
      temp[count] = val;
      count++;
    }
  }

  IntMatrix *result = intmatrix_new(1, count);
  if (count > 0) memcpy(result->data, temp, count * sizeof(long));
  brix_free(temp);
  return result;
}

// --- Reverse ---

Matrix* matrix_reverse(Matrix* m) {
  long total = m->rows * m->cols;
  Matrix *result = matrix_new(m->rows, m->cols);
  for (long i = 0; i < total; i++) {
    result->data[i] = m->data[total - 1 - i];
  }
  return result;
}

IntMatrix* intmatrix_reverse(IntMatrix* m) {
  long total = m->rows * m->cols;
  IntMatrix *result = intmatrix_new(m->rows, m->cols);
  for (long i = 0; i < total; i++) {
    result->data[i] = m->data[total - 1 - i];
  }
  return result;
}

// --- Append/Prepend (returns a new 1x(total+1) array) ---

Matrix* matrix_append(Matrix* m, double val) {
  long total = m->rows * m->cols;
  Matrix *result = matrix_new(1, total + 1);
  if (total > 0) memcpy(result->data, m->data, total * sizeof(double));
  result->data[total] = val;
  return result;
}

IntMatrix* intmatrix_append(IntMatrix* m, long val) {
  long total = m->rows * m->cols;
  IntMatrix *result = intmatrix_new(1, total + 1);
  if (total > 0) memcpy(result->data, m->data, total * sizeof(long));
  result->data[total] = val;
  return result;
}

Matrix* matrix_prepend(Matrix* m, double val) {
  long total = m->rows * m->cols;
  Matrix *result = matrix_new(1, total + 1);
  result->data[0] = val;
  if (total > 0) memcpy(result->data + 1, m->data, total * sizeof(double));
  return result;
}

IntMatrix* intmatrix_prepend(IntMatrix* m, long val) {
  long total = m->rows * m->cols;
  IntMatrix *result = intmatrix_new(1, total + 1);
  result->data[0] = val;
  if (total > 0) memcpy(result->data + 1, m->data, total * sizeof(long));
  return result;
}

// --- Slicing (v1.7 Grupo C) ---
// start/end are flat element indices, start inclusive, end exclusive.
// Result is always a 1-row array (element slicing only, no 2D row
// extraction). Unlike scalar Matrix/IntMatrix indexing elsewhere in this
// runtime (which does no bounds checking at all), start/end here ARE
// clamped to [0, total]: a slice copies a *range* of elements via memcpy,
// so an out-of-range start/end would read past the buffer rather than the
// single stray element a bad scalar index reads.

Matrix* matrix_slice(Matrix* m, long start, long end) {
  long total = m->rows * m->cols;
  if (start < 0) start = 0;
  if (start > total) start = total;
  if (end > total) end = total;
  if (end < start) end = start;
  long len = end - start;
  Matrix* result = matrix_new(1, len);
  if (len > 0) {
    memcpy(result->data, m->data + start, len * sizeof(double));
  }
  return result;
}

IntMatrix* intmatrix_slice(IntMatrix* m, long start, long end) {
  long total = m->rows * m->cols;
  if (start < 0) start = 0;
  if (start > total) start = total;
  if (end > total) end = total;
  if (end < start) end = start;
  long len = end - start;
  IntMatrix* result = intmatrix_new(1, len);
  if (len > 0) {
    memcpy(result->data, m->data + start, len * sizeof(long));
  }
  return result;
}

// ==========================================
// SECTION 1: ERROR HANDLING (v1.1)
// ==========================================

typedef struct {
    char* message;
} BrixError;

// Create a new error with a message
BrixError* brix_error_new(char* msg) {
    if (msg == NULL) {
        return NULL;  // nil error
    }

    BrixError* err = (BrixError*)malloc(sizeof(BrixError));
    if (err == NULL) {
        fprintf(stderr, "Fatal: Failed to allocate memory for error\n");
        exit(1);
    }

    // Copy the message string
    err->message = strdup(msg);
    if (err->message == NULL) {
        fprintf(stderr, "Fatal: Failed to allocate memory for error message\n");
        free(err);
        exit(1);
    }

    return err;
}

// Get error message
char* brix_error_message(BrixError* err) {
    if (err == NULL) {
        return "";  // Empty string for nil error
    }
    return err->message;
}

// Check if error is nil
int brix_error_is_nil(BrixError* err) {
    return err == NULL;
}

// Free error memory
void brix_error_free(BrixError* err) {
    if (err != NULL) {
        if (err->message != NULL) {
            free(err->message);
        }
        free(err);
    }
}

// ==========================================
// SECTION 2: STRINGS (v0.4)
// ==========================================

typedef struct {
  long ref_count;  // ARC reference counting
  long len;
  char *data;
} BrixString;

// Create a new string copying a C literal (e.g: "ola")
BrixString *str_new(char *raw_text) {
  BrixString *s = (BrixString *)malloc(sizeof(BrixString));
  s->ref_count = 1;  // Initialize ARC
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
  s->ref_count = 1;  // Initialize ARC
  s->len = a->len + b->len;

  // Allocate space for both strings
  s->data = (char *)malloc(s->len + 1);

  strcpy(s->data, a->data);
  strcat(s->data, b->data);

  return s;
}

// ARC: Increment reference count
void* string_retain(BrixString* str) {
    if (!str) return NULL;
    str->ref_count++;
    return str;
}

// ARC: Decrement reference count and free if zero
void string_release(BrixString* str) {
    if (!str) return;
    str->ref_count--;

    if (str->ref_count == 0) {
        if (str->data) {
            free(str->data);
        }
        free(str);
    }
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
// SECTION 2.1: STRING FUNCTIONS (v1.1)
// ==========================================

#include <ctype.h>  // For toupper, tolower

// panic(msg) - Print message to stderr and abort the process (v1.7 Grupo H)
void brix_panic(BrixString* msg) {
    fprintf(stderr, "%.*s\n", (int)msg->len, msg->data);
    exit(1);
}

// uppercase(str) - Convert string to uppercase
// Returns new string with all characters in uppercase
BrixString* brix_uppercase(BrixString* str) {
    if (str == NULL || str->data == NULL) {
        return str_new("");
    }

    BrixString* result = (BrixString*)malloc(sizeof(BrixString));
    result->len = str->len;
    result->data = (char*)malloc(result->len + 1);

    for (long i = 0; i < str->len; i++) {
        result->data[i] = toupper((unsigned char)str->data[i]);
    }
    result->data[result->len] = '\0';

    return result;
}

// lowercase(str) - Convert string to lowercase
// Returns new string with all characters in lowercase
BrixString* brix_lowercase(BrixString* str) {
    if (str == NULL || str->data == NULL) {
        return str_new("");
    }

    BrixString* result = (BrixString*)malloc(sizeof(BrixString));
    result->len = str->len;
    result->data = (char*)malloc(result->len + 1);

    for (long i = 0; i < str->len; i++) {
        result->data[i] = tolower((unsigned char)str->data[i]);
    }
    result->data[result->len] = '\0';

    return result;
}

// capitalize(str) - Capitalize first character
// Returns new string with first character uppercase, rest unchanged
BrixString* brix_capitalize(BrixString* str) {
    if (str == NULL || str->data == NULL || str->len == 0) {
        return str_new("");
    }

    BrixString* result = (BrixString*)malloc(sizeof(BrixString));
    result->len = str->len;
    result->data = (char*)malloc(result->len + 1);

    // Copy string
    strcpy(result->data, str->data);

    // Capitalize first character
    result->data[0] = toupper((unsigned char)result->data[0]);

    return result;
}

// byte_size(str) - Get byte size of string
// Returns number of bytes (same as len field)
long brix_byte_size(BrixString* str) {
    if (str == NULL) {
        return 0;
    }
    return str->len;
}

// length(str) - Get number of characters (UTF-8 aware)
// For ASCII this is the same as byte_size
// For UTF-8, counts actual characters not bytes
long brix_length(BrixString* str) {
    if (str == NULL || str->data == NULL) {
        return 0;
    }

    long count = 0;
    for (long i = 0; i < str->len; i++) {
        // UTF-8: Count bytes that don't start with 10xxxxxx
        // This correctly counts multi-byte characters as 1 character
        if ((str->data[i] & 0xC0) != 0x80) {
            count++;
        }
    }
    return count;
}

// replace(str, old, new) - Replace first occurrence
// Returns new string with first occurrence of old replaced by new
BrixString* brix_replace(BrixString* str, BrixString* old, BrixString* new) {
    if (str == NULL || old == NULL || new == NULL) {
        return str;
    }

    if (old->len == 0) {
        return str;  // Can't replace empty string
    }

    // Find first occurrence
    char* pos = strstr(str->data, old->data);
    if (pos == NULL) {
        // Not found, return copy of original
        return str_new(str->data);
    }

    // Calculate new length
    long new_len = str->len - old->len + new->len;
    BrixString* result = (BrixString*)malloc(sizeof(BrixString));
    result->len = new_len;
    result->data = (char*)malloc(new_len + 1);

    // Copy before match
    long before_len = pos - str->data;
    strncpy(result->data, str->data, before_len);

    // Copy replacement
    strcpy(result->data + before_len, new->data);

    // Copy after match
    strcpy(result->data + before_len + new->len, pos + old->len);

    return result;
}

// replace_all(str, old, new) - Replace all occurrences
// Returns new string with all occurrences of old replaced by new
BrixString* brix_replace_all(BrixString* str, BrixString* old, BrixString* new) {
    if (str == NULL || old == NULL || new == NULL) {
        return str;
    }

    if (old->len == 0) {
        return str;  // Can't replace empty string
    }

    // Count occurrences
    long count = 0;
    char* pos = str->data;
    while ((pos = strstr(pos, old->data)) != NULL) {
        count++;
        pos += old->len;
    }

    if (count == 0) {
        // Not found, return copy of original
        return str_new(str->data);
    }

    // Calculate new length
    long new_len = str->len - (count * old->len) + (count * new->len);
    BrixString* result = (BrixString*)malloc(sizeof(BrixString));
    result->len = new_len;
    result->data = (char*)malloc(new_len + 1);

    // Build result with all replacements
    char* src = str->data;
    char* dest = result->data;

    while ((pos = strstr(src, old->data)) != NULL) {
        // Copy before match
        long before_len = pos - src;
        strncpy(dest, src, before_len);
        dest += before_len;

        // Copy replacement
        strcpy(dest, new->data);
        dest += new->len;

        // Move source pointer
        src = pos + old->len;
    }

    // Copy remaining
    strcpy(dest, src);

    return result;
}

// ==========================================
// SECTION 2.2: STRING METHODS (v1.6)
// ==========================================

// trim(str) - Remove leading and trailing whitespace
BrixString* brix_str_trim(BrixString* str) {
    if (str == NULL || str->data == NULL || str->len == 0) {
        return str_new("");
    }
    long start = 0;
    long end = str->len - 1;
    while (start <= end && isspace((unsigned char)str->data[start])) start++;
    while (end > start && isspace((unsigned char)str->data[end])) end--;
    if (start > end) return str_new("");
    long new_len = end - start + 1;
    BrixString* result = (BrixString*)malloc(sizeof(BrixString));
    result->ref_count = 1;
    result->len = new_len;
    result->data = (char*)malloc(new_len + 1);
    strncpy(result->data, str->data + start, new_len);
    result->data[new_len] = '\0';
    return result;
}

// ltrim(str) - Remove leading whitespace
BrixString* brix_str_ltrim(BrixString* str) {
    if (str == NULL || str->data == NULL || str->len == 0) {
        return str_new("");
    }
    long start = 0;
    while (start < str->len && isspace((unsigned char)str->data[start])) start++;
    long new_len = str->len - start;
    BrixString* result = (BrixString*)malloc(sizeof(BrixString));
    result->ref_count = 1;
    result->len = new_len;
    result->data = (char*)malloc(new_len + 1);
    strncpy(result->data, str->data + start, new_len);
    result->data[new_len] = '\0';
    return result;
}

// rtrim(str) - Remove trailing whitespace
BrixString* brix_str_rtrim(BrixString* str) {
    if (str == NULL || str->data == NULL || str->len == 0) {
        return str_new("");
    }
    long end = str->len - 1;
    while (end >= 0 && isspace((unsigned char)str->data[end])) end--;
    long new_len = end + 1;
    if (new_len <= 0) return str_new("");
    BrixString* result = (BrixString*)malloc(sizeof(BrixString));
    result->ref_count = 1;
    result->len = new_len;
    result->data = (char*)malloc(new_len + 1);
    strncpy(result->data, str->data, new_len);
    result->data[new_len] = '\0';
    return result;
}

// starts_with(str, prefix) - Returns 1 if str starts with prefix, 0 otherwise
long brix_str_starts_with(BrixString* str, BrixString* prefix) {
    if (str == NULL || prefix == NULL) return 0;
    if (prefix->len > str->len) return 0;
    if (prefix->len == 0) return 1;
    return strncmp(str->data, prefix->data, prefix->len) == 0 ? 1 : 0;
}

// ends_with(str, suffix) - Returns 1 if str ends with suffix, 0 otherwise
long brix_str_ends_with(BrixString* str, BrixString* suffix) {
    if (str == NULL || suffix == NULL) return 0;
    if (suffix->len > str->len) return 0;
    if (suffix->len == 0) return 1;
    long offset = str->len - suffix->len;
    return strncmp(str->data + offset, suffix->data, suffix->len) == 0 ? 1 : 0;
}

// contains(str, sub) - Returns 1 if str contains sub, 0 otherwise
long brix_str_contains(BrixString* str, BrixString* sub) {
    if (str == NULL || sub == NULL) return 0;
    if (sub->len == 0) return 1;
    return strstr(str->data, sub->data) != NULL ? 1 : 0;
}

// substring(str, start, end) - Returns substring [start, end) exclusive end
BrixString* brix_str_substring(BrixString* str, long start, long end_idx) {
    if (str == NULL || str->data == NULL) return str_new("");
    long len = str->len;
    if (start < 0) start = 0;
    if (end_idx > len) end_idx = len;
    if (start >= end_idx) return str_new("");
    long new_len = end_idx - start;
    BrixString* result = (BrixString*)malloc(sizeof(BrixString));
    result->ref_count = 1;
    result->len = new_len;
    result->data = (char*)malloc(new_len + 1);
    strncpy(result->data, str->data + start, new_len);
    result->data[new_len] = '\0';
    return result;
}

// reverse(str) - Returns reversed string
BrixString* brix_str_reverse(BrixString* str) {
    if (str == NULL || str->data == NULL || str->len == 0) {
        return str_new("");
    }
    BrixString* result = (BrixString*)malloc(sizeof(BrixString));
    result->ref_count = 1;
    result->len = str->len;
    result->data = (char*)malloc(str->len + 1);
    for (long i = 0; i < str->len; i++) {
        result->data[i] = str->data[str->len - 1 - i];
    }
    result->data[str->len] = '\0';
    return result;
}

// repeat(str, n) - Returns str repeated n times
BrixString* brix_str_repeat(BrixString* str, long n) {
    if (str == NULL || str->data == NULL || n <= 0 || str->len == 0) {
        return str_new("");
    }
    long new_len = str->len * n;
    BrixString* result = (BrixString*)malloc(sizeof(BrixString));
    result->ref_count = 1;
    result->len = new_len;
    result->data = (char*)malloc(new_len + 1);
    for (long i = 0; i < n; i++) {
        strncpy(result->data + i * str->len, str->data, str->len);
    }
    result->data[new_len] = '\0';
    return result;
}

// index_of(str, sub) - Returns index of first occurrence, -1 if not found
long brix_str_index_of(BrixString* str, BrixString* sub) {
    if (str == NULL || sub == NULL) return -1;
    if (sub->len == 0) return 0;
    if (sub->len > str->len) return -1;
    char* pos = strstr(str->data, sub->data);
    if (pos == NULL) return -1;
    return (long)(pos - str->data);
}

// char_at(str, idx) - Returns single-character string at position idx
BrixString* brix_str_char_at(BrixString* str, long idx) {
    if (str == NULL || str->data == NULL || idx < 0 || idx >= str->len) {
        return str_new("");
    }
    BrixString* result = (BrixString*)malloc(sizeof(BrixString));
    result->ref_count = 1;
    result->len = 1;
    result->data = (char*)malloc(2);
    result->data[0] = str->data[idx];
    result->data[1] = '\0';
    return result;
}

// ==========================================
// SECTION 2.3: STRING MATRIX (v1.7)
// ==========================================

typedef struct {
  long ref_count;  // ARC reference counting
  long len;
  BrixString** data;  // array of pointers to BrixString
} BrixStringMatrix;

// Create a new StringMatrix with `len` slots, all initialized to NULL
BrixStringMatrix* string_matrix_new(long len) {
    BrixStringMatrix* m = (BrixStringMatrix*)malloc(sizeof(BrixStringMatrix));
    m->ref_count = 1;  // Initialize ARC
    m->len = len;
    m->data = (BrixString**)calloc(len, sizeof(BrixString*));  // calloc zeros pointers (NULL)
    return m;
}

// Get element at index i. Returns NULL if out of bounds or slot is empty.
BrixString* string_matrix_get(BrixStringMatrix* m, long i) {
    if (m == NULL || m->data == NULL) return NULL;
    if (i < 0 || i >= m->len) return NULL;  // out of bounds: return NULL (caller/runtime checks index validity)
    return m->data[i];
}

// Set element at index i, taking co-ownership of `s` via retain.
// Releases any previous non-NULL value at that slot.
void string_matrix_set(BrixStringMatrix* m, long i, BrixString* s) {
    if (m == NULL || m->data == NULL) return;
    if (i < 0 || i >= m->len) return;  // out of bounds: no-op
    if (m->data[i] == s) return;  // self-assignment: no-op, avoids release-then-retain on freed memory
    if (m->data[i] != NULL) {
        string_release(m->data[i]);
    }
    if (s != NULL) {
        string_retain(s);
    }
    m->data[i] = s;
}

// ARC: Increment reference count
void* string_matrix_retain(BrixStringMatrix* m) {
    if (!m) return NULL;
    m->ref_count++;
    return m;
}

// ARC: Decrement reference count and free if zero
void string_matrix_release(BrixStringMatrix* m) {
    if (!m) return;
    m->ref_count--;

    if (m->ref_count == 0) {
        if (m->data) {
            for (long i = 0; i < m->len; i++) {
                if (m->data[i] != NULL) {
                    string_release(m->data[i]);
                }
            }
            free(m->data);
        }
        free(m);
    }
}

// split(str, delim) - Splits str by delim, returning a BrixStringMatrix.
// - If delim->len == 0, each character of str becomes its own element.
// - Consecutive delimiters produce empty strings between them (no collapsing).
// - Empty str with non-empty delim -> StringMatrix of 1 element: [""]
BrixStringMatrix* brix_str_split(BrixString* s, BrixString* delim) {
    if (s == NULL || s->data == NULL) {
        return string_matrix_new(0);
    }

    // Case: split into individual characters
    if (delim == NULL || delim->len == 0) {
        long count = s->len;
        BrixStringMatrix* result = string_matrix_new(count);
        for (long i = 0; i < count; i++) {
            BrixString* ch = (BrixString*)malloc(sizeof(BrixString));
            ch->ref_count = 1;
            ch->len = 1;
            ch->data = (char*)malloc(2);
            ch->data[0] = s->data[i];
            ch->data[1] = '\0';
            result->data[i] = ch;  // already ref_count 1, no need to retain again
        }
        return result;
    }

    // First pass: count how many segments we'll produce
    long count = 1;
    const char* cursor = s->data;
    const char* found;
    while ((found = strstr(cursor, delim->data)) != NULL) {
        count++;
        cursor = found + delim->len;
    }

    BrixStringMatrix* result = string_matrix_new(count);

    // Second pass: fill segments
    cursor = s->data;
    long idx = 0;
    while ((found = strstr(cursor, delim->data)) != NULL) {
        long seg_len = (long)(found - cursor);
        BrixString* seg = (BrixString*)malloc(sizeof(BrixString));
        seg->ref_count = 1;
        seg->len = seg_len;
        seg->data = (char*)malloc(seg_len + 1);
        if (seg_len > 0) {
            memcpy(seg->data, cursor, seg_len);
        }
        seg->data[seg_len] = '\0';
        result->data[idx] = seg;
        idx++;
        cursor = found + delim->len;
    }

    // Final trailing segment (after the last delimiter, or the whole string if none found)
    long seg_len = (long)strlen(cursor);
    BrixString* seg = (BrixString*)malloc(sizeof(BrixString));
    seg->ref_count = 1;
    seg->len = seg_len;
    seg->data = (char*)malloc(seg_len + 1);
    if (seg_len > 0) {
        memcpy(seg->data, cursor, seg_len);
    }
    seg->data[seg_len] = '\0';
    result->data[idx] = seg;

    return result;
}

// join(m, sep) - Concatenates all elements of m with sep between them.
// Returns empty string if m is empty (len == 0).
BrixString* brix_str_join(BrixStringMatrix* m, BrixString* sep) {
    if (m == NULL || m->len == 0) {
        return str_new("");
    }

    long sep_len = (sep != NULL) ? sep->len : 0;
    const char* sep_data = (sep != NULL && sep->data != NULL) ? sep->data : "";

    // Compute total length: sum of element lengths + sep_len * (n - 1)
    long total_len = 0;
    for (long i = 0; i < m->len; i++) {
        BrixString* elem = m->data[i];
        if (elem != NULL && elem->data != NULL) {
            total_len += elem->len;
        }
    }
    total_len += sep_len * (m->len - 1);

    BrixString* result = (BrixString*)malloc(sizeof(BrixString));
    result->ref_count = 1;
    result->len = total_len;
    result->data = (char*)malloc(total_len + 1);

    char* dest = result->data;
    for (long i = 0; i < m->len; i++) {
        BrixString* elem = m->data[i];
        if (elem != NULL && elem->data != NULL && elem->len > 0) {
            memcpy(dest, elem->data, elem->len);
            dest += elem->len;
        }
        if (i < m->len - 1 && sep_len > 0) {
            memcpy(dest, sep_data, sep_len);
            dest += sep_len;
        }
    }
    *dest = '\0';

    return result;
}

// ==========================================
// SECTION 2.4: VECTOR<T> (v1.8 Grupo C)
// ==========================================
//
// Dynamic array with 2x growth. Every element occupies 8 bytes (i64 / f64 /
// BrixString* pointer), so `elem_size` is always 8 in v1.8; `elem_kind` tells
// the runtime whether a slot owns a ref-counted object (a string) so that
// push/set/clear/release can retain/release it correctly. int/float slots are
// plain 8-byte copies.

typedef enum {
  BRIX_VEC_INT = 1,
  BRIX_VEC_FLOAT = 2,
  BRIX_VEC_STRING = 3,
} BrixVectorElemKind;

typedef struct {
  long ref_count;
  long len;
  long cap;
  long elem_size; // bytes per element (8 in v1.8)
  long elem_kind; // BrixVectorElemKind
  void *data;     // contiguous buffer of `cap` slots
} BrixVector;

BrixVector *brix_vector_new(long elem_size, long elem_kind) {
  BrixVector *v = (BrixVector *)malloc(sizeof(BrixVector));
  v->ref_count = 1;
  v->len = 0;
  v->cap = 8;
  v->elem_size = elem_size;
  v->elem_kind = elem_kind;
  v->data = malloc(v->cap * elem_size);
  return v;
}

// Element pointer helper. Unlike Matrix indexing, Vector is an explicit
// dynamic-collection API, so get() bounds-checks and aborts on out-of-range.
void *brix_vector_get_ptr(BrixVector *v, long i) {
  if (i < 0 || i >= v->len) {
    fprintf(stderr, "Error: Vector.get(%ld) out of bounds (len=%ld)\n", i,
            v->len);
    exit(1);
  }
  return (char *)v->data + i * v->elem_size;
}

// push: copy the 8-byte element into the next slot, growing 2x as needed.
// A string element is retained (the vector now co-owns it).
void brix_vector_push(BrixVector *v, void *elem) {
  if (v->len >= v->cap) {
    v->cap *= 2;
    v->data = realloc(v->data, v->cap * v->elem_size);
  }
  char *slot = (char *)v->data + v->len * v->elem_size;
  memcpy(slot, elem, v->elem_size);
  if (v->elem_kind == BRIX_VEC_STRING) {
    string_retain(*(BrixString **)slot);
  }
  v->len++;
}

long brix_vector_len(BrixVector *v) { return v->len; }

// set: overwrite element at index i (0 <= i < len — no append; use push).
// For a string slot, release the old value and retain the new one. Out-of-range
// is a hard error.
void brix_vector_set(BrixVector *v, long i, void *elem) {
  if (i < 0 || i >= v->len) {
    fprintf(stderr, "Error: Vector.set(%ld) out of bounds (len=%ld)\n", i,
            v->len);
    exit(1);
  }
  char *slot = (char *)v->data + i * v->elem_size;
  if (v->elem_kind == BRIX_VEC_STRING) {
    string_release(*(BrixString **)slot); // release the replaced value
  }
  memcpy(slot, elem, v->elem_size);
  if (v->elem_kind == BRIX_VEC_STRING) {
    string_retain(*(BrixString **)slot); // co-own the new value
  }
}

// pop: remove and return the last element. Ownership TRANSFERS to the caller —
// the removed slot is NOT released. Returns 1 and writes the element to `out`,
// or 0 if the vector is empty. For a string vector the popped BrixString* keeps
// its ref_count; the Union(String, Nil) codegen (Phase 4) takes over that owned
// reference.
long brix_vector_pop(BrixVector *v, void *out) {
  if (v->len == 0) {
    return 0;
  }
  v->len--;
  char *slot = (char *)v->data + v->len * v->elem_size;
  memcpy(out, slot, v->elem_size);
  return 1;
}

// clear: reset to empty, releasing owned (string) elements. Capacity and data
// buffer are preserved, so the vector is reusable after clear().
void brix_vector_clear(BrixVector *v) {
  if (v->elem_kind == BRIX_VEC_STRING) {
    for (long i = 0; i < v->len; i++) {
      string_release(*(BrixString **)((char *)v->data + i * v->elem_size));
    }
  }
  v->len = 0;
}

void *brix_vector_retain(BrixVector *v) {
  if (!v)
    return NULL;
  v->ref_count++;
  return v;
}

void brix_vector_release(BrixVector *v) {
  if (!v)
    return;
  v->ref_count--;
  if (v->ref_count == 0) {
    brix_vector_clear(v); // release any owned elements (no duplicated logic)
    free(v->data);
    free(v);
  }
}

// to_array(): convert a Vector<T> to its corresponding native 1D array type.
// Vector<T> has no 2D shape concept, so the result is always a 1xlen row.
// A NULL or empty vector produces a valid, empty (1x0 / len 0) result rather
// than crashing.

// Vector<int> -> IntMatrix (1 x len). Plain value copy, no ARC involved.
IntMatrix *brix_vector_to_intmatrix(BrixVector *v) {
  long len = v ? v->len : 0;
  IntMatrix *result = intmatrix_new(1, len);
  if (len > 0) {
    memcpy(result->data, v->data, len * sizeof(long));
  }
  return result;
}

// Vector<float> -> Matrix (1 x len). Plain value copy, no ARC involved.
Matrix *brix_vector_to_matrix(BrixVector *v) {
  long len = v ? v->len : 0;
  Matrix *result = matrix_new(1, len);
  if (len > 0) {
    memcpy(result->data, v->data, len * sizeof(double));
  }
  return result;
}

// Vector<string> -> BrixStringMatrix. Each BrixString* is retained, so the
// resulting matrix co-owns the strings alongside the original vector: after
// the original vector is cleared/released, the matrix's own reference keeps
// the strings alive.
BrixStringMatrix *brix_vector_to_string_matrix(BrixVector *v) {
  long len = v ? v->len : 0;
  BrixStringMatrix *result = string_matrix_new(len);
  for (long i = 0; i < len; i++) {
    BrixString *s = *(BrixString **)((char *)v->data + i * v->elem_size);
    string_retain(s);
    result->data[i] = s;
  }
  return result;
}

// ==========================================
// SECTION 2.5: QUEUE<T> (v1.8 Grupo D)
// ==========================================
//
// Ring buffer FIFO. Every element occupies 8 bytes (i64 / f64 / BrixString*
// pointer), same convention as BrixVector, and reuses BrixVectorElemKind for
// `elem_kind` so string elements can be retained/released with the existing
// string_retain/string_release helpers. Unlike BrixVector (which only grows
// at the tail), a Queue's physical buffer can wrap around, so growth needs to
// relinearize the logical order before the buffer is doubled (see
// brix_queue_grow below).

typedef struct {
  long ref_count;
  long head;      // index of the first logical element
  long tail;      // index of the next free slot for enqueue
  long len;       // number of elements currently in the queue
  long cap;       // physical buffer capacity
  long elem_size; // bytes per element (8 in v1.8)
  long elem_kind; // BrixVectorElemKind
  void *data;     // contiguous physical buffer of `cap` slots
} BrixQueue;

BrixQueue *brix_queue_new(long elem_size, long elem_kind) {
  BrixQueue *q = (BrixQueue *)malloc(sizeof(BrixQueue));
  q->ref_count = 1;
  q->head = 0;
  q->tail = 0;
  q->len = 0;
  q->cap = 4;
  q->elem_size = elem_size;
  q->elem_kind = elem_kind;
  q->data = malloc(q->cap * elem_size);
  return q;
}

// Relinearize the ring buffer into a new, doubled-capacity buffer: copy the
// `len` elements in logical order (starting at `head`, wrapping around) into
// physical slots [0, len) of the new buffer, then reset head=0, tail=len.
// Called only when the buffer is full (len == cap) and another enqueue needs
// room.
static void brix_queue_grow(BrixQueue *q) {
  long new_cap = q->cap * 2;
  void *new_data = malloc(new_cap * q->elem_size);
  for (long i = 0; i < q->len; i++) {
    long src_idx = (q->head + i) % q->cap;
    memcpy((char *)new_data + i * q->elem_size,
           (char *)q->data + src_idx * q->elem_size, q->elem_size);
  }
  free(q->data);
  q->data = new_data;
  q->cap = new_cap;
  q->head = 0;
  q->tail = q->len;
}

// enqueue: write into the tail slot, advance tail with wraparound, grow the
// buffer first if it's full. A string element is retained (the queue now
// co-owns it).
void brix_queue_enqueue(BrixQueue *q, void *elem) {
  if (q->len == q->cap) {
    brix_queue_grow(q);
  }
  char *slot = (char *)q->data + q->tail * q->elem_size;
  memcpy(slot, elem, q->elem_size);
  if (q->elem_kind == BRIX_VEC_STRING) {
    string_retain(*(BrixString **)slot);
  }
  q->tail = (q->tail + 1) % q->cap;
  q->len++;
}

// dequeue: remove and return the front element. Ownership TRANSFERS to the
// caller — the removed slot is NOT released. Returns 1 and writes the
// element to `out`, or 0 if the queue is empty.
long brix_queue_dequeue(BrixQueue *q, void *out) {
  if (q->len == 0) {
    return 0;
  }
  char *slot = (char *)q->data + q->head * q->elem_size;
  memcpy(out, slot, q->elem_size);
  q->head = (q->head + 1) % q->cap;
  q->len--;
  return 1;
}

// front: pointer to the slot at `head`, for the codegen side to load (and
// retain, for a string element) — same division of labor as
// brix_vector_get_ptr. Aborts if the queue is empty.
void *brix_queue_front(BrixQueue *q) {
  if (q->len == 0) {
    fprintf(stderr, "Error: Queue.front() called on empty queue\n");
    exit(1);
  }
  return (char *)q->data + q->head * q->elem_size;
}

long brix_queue_size(BrixQueue *q) { return q->len; }

void *brix_queue_retain(BrixQueue *q) {
  if (!q)
    return NULL;
  q->ref_count++;
  return q;
}

void brix_queue_release(BrixQueue *q) {
  if (!q)
    return;
  q->ref_count--;
  if (q->ref_count == 0) {
    if (q->elem_kind == BRIX_VEC_STRING) {
      for (long i = 0; i < q->len; i++) {
        long idx = (q->head + i) % q->cap;
        string_release(*(BrixString **)((char *)q->data + idx * q->elem_size));
      }
    }
    free(q->data);
    free(q);
  }
}

// ==========================================
// SECTION 2.6: HEAP<T> (v1.8 Grupo E)
// ==========================================
//
// MinHeap<T>/MaxHeap<T> are literally a BrixVector* underneath (same idea as
// Stack<T> in Grupo D — reuses brix_vector_new/retain/release/len with no new
// struct). The only difference between Min and Max is the DIRECTION of the
// comparison, decided by an `int is_max` parameter passed on every call
// (0=Min, 1=Max) — it is never stored on the struct itself; the Rust codegen
// side knows statically whether it's compiling MinHeap or MaxHeap and passes
// the right constant.
//
// Binary heap invariant is maintained over the vector's flat buffer using the
// usual array-heap indexing: children of `idx` are at `2*idx+1`/`2*idx+2`,
// parent of `idx` is at `(idx-1)/2`.

// Compares elements at index i and j (natural ascending order, ignores
// is_max). Returns <0 if element i is "smaller" than element j, 0 if equal,
// >0 if larger.
static int brix_heap_compare(BrixVector *h, long i, long j) {
  char *a = (char *)h->data + i * h->elem_size;
  char *b = (char *)h->data + j * h->elem_size;
  if (h->elem_kind == BRIX_VEC_INT) {
    long av = *(long *)a, bv = *(long *)b;
    return (av > bv) - (av < bv);
  } else if (h->elem_kind == BRIX_VEC_FLOAT) {
    double av = *(double *)a, bv = *(double *)b;
    return (av > bv) - (av < bv);
  } else { // BRIX_VEC_STRING
    BrixString *as = *(BrixString **)a;
    BrixString *bs = *(BrixString **)b;
    return strcmp(as->data, bs->data);
  }
}

// "better(i, j)": true if element i should sit closer to the root than
// element j, given is_max (0 = MinHeap: smallest first; 1 = MaxHeap: largest
// first).
static int brix_heap_better(BrixVector *h, long i, long j, int is_max) {
  int cmp = brix_heap_compare(h, i, j);
  return is_max ? (cmp > 0) : (cmp < 0);
}

// Raw swap of 2 slots (memcpy through an 8-byte scratch buffer — elem_size is
// always 8). Does NOT touch ref counts: both slots keep holding the same
// references, only their physical position changes, so the total number of
// live references doesn't change — no string_retain/release needed (or
// wanted) here.
static void brix_heap_swap(BrixVector *h, long i, long j) {
  char tmp[8];
  char *a = (char *)h->data + i * h->elem_size;
  char *b = (char *)h->data + j * h->elem_size;
  memcpy(tmp, a, h->elem_size);
  memcpy(a, b, h->elem_size);
  memcpy(b, tmp, h->elem_size);
}

static void brix_heap_sift_up(BrixVector *h, long idx, int is_max) {
  while (idx > 0) {
    long parent = (idx - 1) / 2;
    if (brix_heap_better(h, idx, parent, is_max)) {
      brix_heap_swap(h, idx, parent);
      idx = parent;
    } else {
      break;
    }
  }
}

static void brix_heap_sift_down(BrixVector *h, long idx, int is_max) {
  while (1) {
    long left = 2 * idx + 1;
    long right = 2 * idx + 2;
    long best = idx;
    if (left < h->len && brix_heap_better(h, left, best, is_max))
      best = left;
    if (right < h->len && brix_heap_better(h, right, best, is_max))
      best = right;
    if (best == idx)
      break;
    brix_heap_swap(h, idx, best);
    idx = best;
  }
}

// push: reuses brix_vector_push (append + 2x grow + string_retain already
// handled there — no duplicated ARC logic here), then restores the heap
// property by sifting the freshly-appended element up.
void brix_heap_push(BrixVector *h, void *elem, int is_max) {
  brix_vector_push(h, elem);
  brix_heap_sift_up(h, h->len - 1, is_max);
}

// pop: removes and returns the root (index 0). Ownership of the root
// TRANSFERS to the caller — the root slot is not released (same contract as
// brix_vector_pop). Moving the last element into the root is done with a raw
// memcpy, not brix_vector_set (which would retain/release and break the
// ownership transfer: the root we're returning still holds its original
// reference intact, without being duplicated or lost). The vector remains
// owner of exactly the elements left in the heap.
long brix_heap_pop(BrixVector *h, void *out, int is_max) {
  if (h->len == 0) {
    return 0;
  }
  char *root = (char *)h->data;
  memcpy(out, root, h->elem_size); // ownership transfers to caller, no release
  h->len--;
  if (h->len > 0) {
    char *last = (char *)h->data + h->len * h->elem_size;
    memcpy(root, last, h->elem_size); // raw move, no retain/release
  }
  brix_heap_sift_down(h, 0, is_max);
  return 1;
}

// peek: pointer to the root slot (index 0), for the codegen side to load (and
// retain, for a string element) — same division of labor as
// brix_vector_get_ptr/brix_queue_front. Aborts with its OWN message if empty
// (does not reuse brix_vector_get_ptr's generic message).
void *brix_heap_peek(BrixVector *h) {
  if (h->len == 0) {
    fprintf(stderr, "Error: Heap.peek() called on empty heap\n");
    exit(1);
  }
  return h->data;
}

// ==========================================
// SECTION 2.7: HASHMAP<K,V> (v1.8 Grupo F)
// ==========================================
//
// Open-addressing hash table (linear probing) with tombstone deletion. Keys:
// int/string. Values: int/float/string — restricted to the same closed set
// as the other containers (Vector/Stack/Queue/Heap), since "any Brix type"
// would need runtime-dispatched ARC that the compiler doesn't have.
//
// Every slot's key/value is a raw 8-byte word: an i64 (int/float bits) or a
// BrixString* reinterpreted through that word, exactly like BrixVector's
// elem_size==8 slots. A HashEntry additionally tracks `occupied`/`deleted`
// to distinguish 3 slot states: NEVER-USED (0,0) — probing stops here, key
// doesn't exist; LIVE (1,0); TOMBSTONE (0,1) — probing continues past it
// (the key may exist further along the probe chain), but it's available for
// reuse on insert.

typedef struct {
  long key;      // slot de 8 bytes: i64 cru OU BrixString* (reinterpretado via cast)
  long value;    // slot de 8 bytes: i64/f64 (bits) OU BrixString*
  int occupied;  // 1 = entry viva
  int deleted;   // 1 = tombstone (foi ocupada, removida)
} HashEntry;

typedef struct {
  long ref_count;
  long len;      // entries vivas (não-tombstone)
  long cap;      // capacidade física de `entries`
  long used;     // occupied + tombstone (usado só pra decidir rehash, não pra iteração)
  long key_kind; // BRIX_VEC_INT ou BRIX_VEC_STRING
  long val_kind; // BRIX_VEC_INT, BRIX_VEC_FLOAT ou BRIX_VEC_STRING
  HashEntry *entries;
} BrixHashMap;

// Hash de `key` (interpretado conforme m->key_kind). Int: multiplicative
// hash (Knuth). String: FNV-1a sobre o conteúdo (não o ponteiro) — necessário
// pra que duas BrixString* diferentes com o mesmo texto colidam no mesmo
// bucket.
static long brix_hashmap_hash(BrixHashMap *m, void *key) {
  if (m->key_kind == BRIX_VEC_INT) {
    unsigned long uk = (unsigned long)(*(long *)key);
    uk = uk * 2654435761UL; // Knuth multiplicative hash
    return (long)uk;
  } else { // BRIX_VEC_STRING
    BrixString *s = *(BrixString **)key;
    unsigned long h = 1469598103934665603UL; // FNV-1a offset basis
    for (char *p = s->data; *p; p++) {
      h ^= (unsigned char)*p;
      h *= 1099511628211UL; // FNV prime
    }
    return (long)h;
  }
}

// Igualdade de CONTEÚDO, não de ponteiro — crucial pra strings: duas
// BrixString* diferentes com o mesmo texto devem contar como a mesma chave.
static int brix_hashmap_key_eq(BrixHashMap *m, long a, long b) {
  if (m->key_kind == BRIX_VEC_INT) {
    return a == b;
  } else {
    BrixString *as = (BrixString *)a;
    BrixString *bs = (BrixString *)b;
    return strcmp(as->data, bs->data) == 0;
  }
}

// Busca `key` (passado como ponteiro pro slot de 8 bytes do CALLER, não um
// long cru — trate como *(long*)key pra obter o valor de 8 bytes, seja ele
// um i64 ou um BrixString* reinterpretado).
// Se encontrar entry VIVA com chave igual: retorna 1, *found_idx = índice.
// Se não encontrar: retorna 0. *insert_idx (se não-NULL) recebe o primeiro
// slot disponível (tombstone OU nunca-usado) encontrado no caminho da busca
// — usado só por set() pra saber onde inserir.
static int brix_hashmap_find(BrixHashMap *m, void *key_ptr, long *found_idx,
                              long *insert_idx) {
  long key = *(long *)key_ptr;
  long hash = brix_hashmap_hash(m, key_ptr);
  long idx = ((unsigned long)hash) % (unsigned long)m->cap;
  long first_avail = -1;
  for (long probes = 0; probes < m->cap; probes++) {
    HashEntry *e = &m->entries[idx];
    if (!e->occupied && !e->deleted) {
      // nunca usado -> a busca termina aqui, chave não existe
      if (insert_idx)
        *insert_idx = (first_avail != -1) ? first_avail : idx;
      return 0;
    }
    if (e->occupied && !e->deleted) {
      if (brix_hashmap_key_eq(m, e->key, key)) {
        if (found_idx)
          *found_idx = idx;
        return 1;
      }
    } else if (e->deleted && first_avail == -1) {
      first_avail = idx;
    }
    idx = (idx + 1) % m->cap;
  }
  // não deveria alcançar isso dado a política de load factor, mas guarda:
  if (insert_idx)
    *insert_idx = first_avail;
  return 0;
}

BrixHashMap *brix_hashmap_new(long key_kind, long val_kind) {
  BrixHashMap *m = (BrixHashMap *)malloc(sizeof(BrixHashMap));
  m->ref_count = 1;
  m->len = 0;
  m->cap = 8;
  m->used = 0;
  m->key_kind = key_kind;
  m->val_kind = val_kind;
  // calloc zera key/value/occupied/deleted -> todos os slots começam "nunca usados"
  m->entries = (HashEntry *)calloc(m->cap, sizeof(HashEntry));
  return m;
}

// Relinariza pra um array novo de capacidade `new_cap`, reinserindo só as
// entries VIVAS (tombstones são descartados). NÃO chama string_retain/release
// — é uma relocação pura, a posse não muda (mesma ideia do growth do Queue).
static void brix_hashmap_rehash(BrixHashMap *m, long new_cap) {
  HashEntry *old_entries = m->entries;
  long old_cap = m->cap;
  HashEntry *new_entries = (HashEntry *)calloc(new_cap, sizeof(HashEntry));
  m->entries = new_entries;
  m->cap = new_cap;
  m->len = 0;
  m->used = 0;
  for (long i = 0; i < old_cap; i++) {
    HashEntry *e = &old_entries[i];
    if (e->occupied && !e->deleted) {
      long hash = brix_hashmap_hash(m, &e->key);
      long idx = ((unsigned long)hash) % (unsigned long)new_cap;
      while (new_entries[idx].occupied) {
        idx = (idx + 1) % new_cap;
      }
      new_entries[idx].key = e->key;
      new_entries[idx].value = e->value;
      new_entries[idx].occupied = 1;
      new_entries[idx].deleted = 0;
      m->len++;
      m->used++;
    }
  }
  free(old_entries);
}

// set: insere (chave nova) ou sobrescreve (chave existente).
//
// CONTRATO DE ARC (importante, não simplifique):
// - Chave NOVA: retém a chave (se string) e o valor (se string). O codegen do
//   lado Rust vai liberar o temp de origem depois, se for owned — a MESMA
//   regra do Vector.push já aplicada independentemente pra key e value.
// - Chave EXISTENTE (mesmo conteúdo, mesmo com um BrixString* diferente do
//   armazenado): NÃO toca na chave armazenada (mantém a referência original,
//   NÃO substitui, NÃO retém a chave recebida) — só libera o valor antigo (se
//   string) e retém o valor novo (se string). O ponteiro de chave recebido,
//   se era um temp owned do lado do codegen, será liberado por lá mesmo
//   assim — como esta função não o retém nesse caso, o release do codegen o
//   libera corretamente (sem leak, sem afetar a chave armazenada).
void brix_hashmap_set(BrixHashMap *m, void *key_ptr, void *val_ptr) {
  long found_idx, insert_idx;
  if (brix_hashmap_find(m, key_ptr, &found_idx, &insert_idx)) {
    HashEntry *e = &m->entries[found_idx];
    if (m->val_kind == BRIX_VEC_STRING) {
      string_release((BrixString *)e->value);
    }
    e->value = *(long *)val_ptr;
    if (m->val_kind == BRIX_VEC_STRING) {
      string_retain((BrixString *)e->value);
    }
    return;
  }
  HashEntry *e = &m->entries[insert_idx];
  int was_never_used = !e->occupied && !e->deleted;
  e->key = *(long *)key_ptr;
  e->value = *(long *)val_ptr;
  if (m->key_kind == BRIX_VEC_STRING) {
    string_retain((BrixString *)e->key);
  }
  if (m->val_kind == BRIX_VEC_STRING) {
    string_retain((BrixString *)e->value);
  }
  e->occupied = 1;
  e->deleted = 0;
  m->len++;
  if (was_never_used) {
    m->used++;
  }
  // Rehash: dispara quando used*10 >= cap*7. Cresce (cap*2) só se as entries
  // vivas sozinhas já ocupariam metade da capacidade ANTIGA; senão, rehash na
  // MESMA capacidade só pra descartar tombstones acumulados.
  if (m->used * 10 >= m->cap * 7) {
    long new_cap = (m->len * 2 >= m->cap) ? m->cap * 2 : m->cap;
    brix_hashmap_rehash(m, new_cap);
  }
}

// get: sucesso retorna 1 e escreve o valor em `out` (retido ANTES de
// escrever, se for string — contrato deliberado: sucesso sempre retorna
// OWNED, o codegen não precisa retentar de novo). Falha (chave ausente)
// retorna 0 e não toca em `out`.
int brix_hashmap_get(BrixHashMap *m, void *key_ptr, void *out) {
  long found_idx;
  if (!brix_hashmap_find(m, key_ptr, &found_idx, NULL)) {
    return 0;
  }
  HashEntry *e = &m->entries[found_idx];
  if (m->val_kind == BRIX_VEC_STRING) {
    string_retain((BrixString *)e->value); // get() bem-sucedido retorna OWNED
  }
  *(long *)out = e->value;
  return 1;
}

int brix_hashmap_has(BrixHashMap *m, void *key_ptr) {
  long found_idx;
  return brix_hashmap_find(m, key_ptr, &found_idx, NULL);
}

// delete: idempotente — no-op se a chave não existe (NÃO é erro, não aborta
// nem imprime nada).
void brix_hashmap_delete(BrixHashMap *m, void *key_ptr) {
  long found_idx;
  if (!brix_hashmap_find(m, key_ptr, &found_idx, NULL)) {
    return; // no-op — comportamento intencional
  }
  HashEntry *e = &m->entries[found_idx];
  if (m->key_kind == BRIX_VEC_STRING) {
    string_release((BrixString *)e->key);
  }
  if (m->val_kind == BRIX_VEC_STRING) {
    string_release((BrixString *)e->value);
  }
  e->occupied = 0;
  e->deleted = 1;
  m->len--;
  // `used` NÃO diminui — o tombstone continua contando pra disparar rehash de
  // compactação eventualmente.
}

long brix_hashmap_len(BrixHashMap *m) { return m->len; }

// keys_str: só chamado quando key_kind==BRIX_VEC_STRING. Retém cada chave
// (co-ownership com o mapa — mesma lógica de brix_vector_to_string_matrix).
BrixStringMatrix *brix_hashmap_keys_str(BrixHashMap *m) {
  BrixStringMatrix *result = string_matrix_new(m->len);
  long out_i = 0;
  for (long i = 0; i < m->cap; i++) {
    HashEntry *e = &m->entries[i];
    if (e->occupied && !e->deleted) {
      BrixString *s = (BrixString *)e->key;
      string_retain(s);
      result->data[out_i++] = s;
    }
  }
  return result;
}

// keys_int: só chamado quando key_kind==BRIX_VEC_INT.
IntMatrix *brix_hashmap_keys_int(BrixHashMap *m) {
  IntMatrix *result = intmatrix_new(1, m->len);
  long out_i = 0;
  for (long i = 0; i < m->cap; i++) {
    HashEntry *e = &m->entries[i];
    if (e->occupied && !e->deleted) {
      result->data[out_i++] = e->key;
    }
  }
  return result;
}

void *brix_hashmap_retain(BrixHashMap *m) {
  if (!m)
    return NULL;
  m->ref_count++;
  return m;
}

void brix_hashmap_release(BrixHashMap *m) {
  if (!m)
    return;
  m->ref_count--;
  if (m->ref_count == 0) {
    for (long i = 0; i < m->cap; i++) {
      HashEntry *e = &m->entries[i];
      if (e->occupied && !e->deleted) {
        if (m->key_kind == BRIX_VEC_STRING)
          string_release((BrixString *)e->key);
        if (m->val_kind == BRIX_VEC_STRING)
          string_release((BrixString *)e->value);
      }
    }
    free(m->entries);
    free(m);
  }
}

// ==========================================
// SECTION 2.8: DATETIME (v1.9 Grupo A)
// ==========================================

typedef struct {
    long ref_count;
    int year;
    int month;
    int day;
    int hour;
    int minute;
    int second;
    int tz_offset_minutes;
} BrixDateTime;

BrixDateTime* datetime_new(int year, int month, int day, int hour, int minute, int second, int tz_offset_minutes) {
    BrixDateTime* dt = (BrixDateTime*)brix_malloc(sizeof(BrixDateTime));
    dt->ref_count = 1;
    dt->year = year;
    dt->month = month;
    dt->day = day;
    dt->hour = hour;
    dt->minute = minute;
    dt->second = second;
    dt->tz_offset_minutes = tz_offset_minutes;
    return dt;
}

BrixDateTime* datetime_retain(BrixDateTime* dt) {
    if (dt) {
        dt->ref_count++;
    }
    return dt;
}

void datetime_release(BrixDateTime* dt) {
    if (dt) {
        dt->ref_count--;
        if (dt->ref_count <= 0) {
            brix_free(dt);
        }
    }
}

static int get_local_tz_offset_minutes(time_t rawtime, struct tm* ltime) {
#if defined(__USE_MISC) || defined(_BSD_SOURCE) || defined(__APPLE__)
    return (int)(ltime->tm_gmtoff / 60);
#else
    struct tm gtime;
    gmtime_r(&rawtime, &gtime);
    time_t g_epoch = mktime(&gtime);
    time_t l_epoch = mktime(ltime);
    return (int)(difftime(l_epoch, g_epoch) / 60);
#endif
}

BrixDateTime* datetime_now(void) {
    time_t rawtime = time(NULL);
    struct tm ltime;
    localtime_r(&rawtime, &ltime);
    int tz_off = get_local_tz_offset_minutes(rawtime, &ltime);
    return datetime_new(ltime.tm_year + 1900, ltime.tm_mon + 1, ltime.tm_mday,
                        ltime.tm_hour, ltime.tm_min, ltime.tm_sec, tz_off);
}

BrixDateTime* datetime_today(void) {
    time_t rawtime = time(NULL);
    struct tm ltime;
    localtime_r(&rawtime, &ltime);
    int tz_off = get_local_tz_offset_minutes(rawtime, &ltime);
    return datetime_new(ltime.tm_year + 1900, ltime.tm_mon + 1, ltime.tm_mday,
                        0, 0, 0, tz_off);
}

long datetime_timestamp(BrixDateTime* dt) {
    if (!dt) return 0;
    struct tm t;
    memset(&t, 0, sizeof(struct tm));
    t.tm_year = dt->year - 1900;
    t.tm_mon = dt->month - 1;
    t.tm_mday = dt->day;
    t.tm_hour = dt->hour;
    t.tm_min = dt->minute;
    t.tm_sec = dt->second;
    t.tm_isdst = -1;
    time_t ts = mktime(&t);
    return (long)ts;
}

BrixDateTime* datetime_from_timestamp(long ts) {
    time_t rawtime = (time_t)ts;
    struct tm ltime;
    localtime_r(&rawtime, &ltime);
    int tz_off = get_local_tz_offset_minutes(rawtime, &ltime);
    return datetime_new(ltime.tm_year + 1900, ltime.tm_mon + 1, ltime.tm_mday,
                        ltime.tm_hour, ltime.tm_min, ltime.tm_sec, tz_off);
}

BrixString* datetime_format(BrixDateTime* dt, BrixString* fmt) {
    if (!dt || !fmt || !fmt->data) {
        return str_new("");
    }
    char buf[1024];
    buf[0] = '\0';
    size_t out_len = 0;
    size_t fmt_len = fmt->len;
    const char* pattern = fmt->data;

    size_t i = 0;
    while (i < fmt_len && out_len + 1 < sizeof(buf)) {
        size_t rem = sizeof(buf) - out_len;
        if (i + 4 <= fmt_len && memcmp(pattern + i, "YYYY", 4) == 0) {
            int n = snprintf(buf + out_len, rem, "%04d", dt->year);
            if (n > 0) {
                out_len += ((size_t)n < rem ? (size_t)n : rem - 1);
            }
            i += 4;
        } else if (i + 2 <= fmt_len && memcmp(pattern + i, "MM", 2) == 0) {
            int n = snprintf(buf + out_len, rem, "%02d", dt->month);
            if (n > 0) {
                out_len += ((size_t)n < rem ? (size_t)n : rem - 1);
            }
            i += 2;
        } else if (i + 2 <= fmt_len && memcmp(pattern + i, "DD", 2) == 0) {
            int n = snprintf(buf + out_len, rem, "%02d", dt->day);
            if (n > 0) {
                out_len += ((size_t)n < rem ? (size_t)n : rem - 1);
            }
            i += 2;
        } else if (i + 2 <= fmt_len && memcmp(pattern + i, "HH", 2) == 0) {
            int n = snprintf(buf + out_len, rem, "%02d", dt->hour);
            if (n > 0) {
                out_len += ((size_t)n < rem ? (size_t)n : rem - 1);
            }
            i += 2;
        } else if (i + 2 <= fmt_len && memcmp(pattern + i, "mm", 2) == 0) {
            int n = snprintf(buf + out_len, rem, "%02d", dt->minute);
            if (n > 0) {
                out_len += ((size_t)n < rem ? (size_t)n : rem - 1);
            }
            i += 2;
        } else if (i + 2 <= fmt_len && memcmp(pattern + i, "ss", 2) == 0) {
            int n = snprintf(buf + out_len, rem, "%02d", dt->second);
            if (n > 0) {
                out_len += ((size_t)n < rem ? (size_t)n : rem - 1);
            }
            i += 2;
        } else {
            buf[out_len++] = pattern[i];
            buf[out_len] = '\0';
            i++;
        }
    }
    return str_new(buf);
}

BrixDateTime* datetime_parse(BrixString* str, BrixString* fmt) {
    if (!str || !fmt || !str->data || !fmt->data) {
        return NULL;
    }
    int year = 1970, month = 1, day = 1, hour = 0, minute = 0, second = 0;
    const char* s = str->data;
    const char* f = fmt->data;
    size_t s_len = str->len;
    size_t f_len = fmt->len;
    size_t si = 0, fi = 0;

    while (fi < f_len && si < s_len) {
        if (fi + 4 <= f_len && memcmp(f + fi, "YYYY", 4) == 0) {
            if (si + 4 > s_len) return NULL;
            int val = 0;
            for (int k = 0; k < 4; k++) {
                if (s[si + k] < '0' || s[si + k] > '9') return NULL;
                val = val * 10 + (s[si + k] - '0');
            }
            year = val;
            fi += 4;
            si += 4;
        } else if (fi + 2 <= f_len && memcmp(f + fi, "MM", 2) == 0) {
            if (si + 2 > s_len) return NULL;
            int val = 0;
            for (int k = 0; k < 2; k++) {
                if (s[si + k] < '0' || s[si + k] > '9') return NULL;
                val = val * 10 + (s[si + k] - '0');
            }
            month = val;
            fi += 2;
            si += 2;
        } else if (fi + 2 <= f_len && memcmp(f + fi, "DD", 2) == 0) {
            if (si + 2 > s_len) return NULL;
            int val = 0;
            for (int k = 0; k < 2; k++) {
                if (s[si + k] < '0' || s[si + k] > '9') return NULL;
                val = val * 10 + (s[si + k] - '0');
            }
            day = val;
            fi += 2;
            si += 2;
        } else if (fi + 2 <= f_len && memcmp(f + fi, "HH", 2) == 0) {
            if (si + 2 > s_len) return NULL;
            int val = 0;
            for (int k = 0; k < 2; k++) {
                if (s[si + k] < '0' || s[si + k] > '9') return NULL;
                val = val * 10 + (s[si + k] - '0');
            }
            hour = val;
            fi += 2;
            si += 2;
        } else if (fi + 2 <= f_len && memcmp(f + fi, "mm", 2) == 0) {
            if (si + 2 > s_len) return NULL;
            int val = 0;
            for (int k = 0; k < 2; k++) {
                if (s[si + k] < '0' || s[si + k] > '9') return NULL;
                val = val * 10 + (s[si + k] - '0');
            }
            minute = val;
            fi += 2;
            si += 2;
        } else if (fi + 2 <= f_len && memcmp(f + fi, "ss", 2) == 0) {
            if (si + 2 > s_len) return NULL;
            int val = 0;
            for (int k = 0; k < 2; k++) {
                if (s[si + k] < '0' || s[si + k] > '9') return NULL;
                val = val * 10 + (s[si + k] - '0');
            }
            second = val;
            fi += 2;
            si += 2;
        } else {
            if (s[si] != f[fi]) return NULL;
            si++;
            fi++;
        }
    }
    if (fi < f_len || si < s_len) return NULL;
    if (month < 1 || month > 12 || day < 1 || day > 31 || hour < 0 || hour > 23 || minute < 0 || minute > 59 || second < 0 || second > 59) {
        return NULL;
    }

    struct tm t;
    memset(&t, 0, sizeof(struct tm));
    t.tm_year = year - 1900;
    t.tm_mon = month - 1;
    t.tm_mday = day;
    t.tm_hour = hour;
    t.tm_min = minute;
    t.tm_sec = second;
    t.tm_isdst = -1;

    time_t ts = mktime(&t);
    if (ts == (time_t)-1 && (year != 1969 && year != 1970)) {
        return NULL;
    }
    if (t.tm_year + 1900 != year || t.tm_mon + 1 != month || t.tm_mday != day ||
        t.tm_hour != hour || t.tm_min != minute || t.tm_sec != second) {
        return NULL;
    }

    struct tm ltime;
    localtime_r(&ts, &ltime);
    int tz_off = get_local_tz_offset_minutes(ts, &ltime);
    return datetime_new(year, month, day, hour, minute, second, tz_off);
}

BrixDateTime* datetime_add_days(BrixDateTime* dt, long n) {
    if (!dt) return NULL;
    time_t ts = (time_t)datetime_timestamp(dt);
    ts += (n * 86400);
    return datetime_from_timestamp((long)ts);
}

BrixDateTime* datetime_add_hours(BrixDateTime* dt, long n) {
    if (!dt) return NULL;
    time_t ts = (time_t)datetime_timestamp(dt);
    ts += (n * 3600);
    return datetime_from_timestamp((long)ts);
}

BrixDateTime* datetime_add_minutes(BrixDateTime* dt, long n) {
    if (!dt) return NULL;
    time_t ts = (time_t)datetime_timestamp(dt);
    ts += (n * 60);
    return datetime_from_timestamp((long)ts);
}

BrixDateTime* datetime_add_seconds(BrixDateTime* dt, long n) {
    if (!dt) return NULL;
    time_t ts = (time_t)datetime_timestamp(dt);
    ts += n;
    return datetime_from_timestamp((long)ts);
}

long datetime_diff_seconds(BrixDateTime* a, BrixDateTime* b) {
    if (!a || !b) return 0;
    long ts_a = datetime_timestamp(a);
    long ts_b = datetime_timestamp(b);
    return ts_b - ts_a;
}

int datetime_compare(BrixDateTime* a, BrixDateTime* b) {
    if (a == b) return 0;
    if (!a) return -1;
    if (!b) return 1;
    long ts_a = datetime_timestamp(a);
    long ts_b = datetime_timestamp(b);
    if (ts_a < ts_b) return -1;
    if (ts_a > ts_b) return 1;
    return 0;
}


// ==========================================
// SECTION 2.9: JSON (v1.9 Grupo B)
// ==========================================

typedef enum {
    JSON_NULL   = 0,
    JSON_BOOL   = 1,
    JSON_INT    = 2,
    JSON_FLOAT  = 3,
    JSON_STRING = 4,
    JSON_ARRAY  = 5,
    JSON_OBJECT = 6,
} JsonTag;

struct JsonValue;

typedef struct JsonObjectEntry {
    BrixString* key;
    struct JsonValue* val;
    struct JsonObjectEntry* next;
} JsonObjectEntry;

typedef struct {
    JsonObjectEntry* head;
    JsonObjectEntry* tail;
    size_t count;
} JsonObject;

typedef struct {
    size_t len;
    size_t cap;
    struct JsonValue** items;
} JsonArray;

typedef struct JsonValue {
    long ref_count;
    JsonTag tag;
    union {
        long          as_bool;
        long          as_int;
        double        as_float;
        BrixString*   as_string;
        JsonArray*    as_array;
        JsonObject*   as_object;
    } data;
} JsonValue;

JsonValue* json_retain(JsonValue* v);
void json_release(JsonValue* v);

static JsonValue* json_new(JsonTag tag) {
    JsonValue* v = (JsonValue*)malloc(sizeof(JsonValue));
    if (!v) return NULL;
    v->ref_count = 1;
    v->tag = tag;
    memset(&v->data, 0, sizeof(v->data));
    return v;
}

JsonValue* json_retain(JsonValue* v) {
    if (v) {
        v->ref_count++;
    }
    return v;
}

void json_release(JsonValue* v) {
    if (!v) return;
    v->ref_count--;
    if (v->ref_count <= 0) {
        if (v->tag == JSON_STRING && v->data.as_string) {
            string_release(v->data.as_string);
        } else if (v->tag == JSON_ARRAY && v->data.as_array) {
            JsonArray* arr = v->data.as_array;
            for (size_t i = 0; i < arr->len; i++) {
                if (arr->items[i]) {
                    json_release(arr->items[i]);
                }
            }
            free(arr->items);
            free(arr);
        } else if (v->tag == JSON_OBJECT && v->data.as_object) {
            JsonObject* obj = v->data.as_object;
            JsonObjectEntry* curr = obj->head;
            while (curr) {
                JsonObjectEntry* next = curr->next;
                if (curr->key) string_release(curr->key);
                if (curr->val) json_release(curr->val);
                free(curr);
                curr = next;
            }
            free(obj);
        }
        free(v);
    }
}

JsonValue* json_null(void) {
    return json_new(JSON_NULL);
}

JsonValue* json_bool(long v) {
    JsonValue* val = json_new(JSON_BOOL);
    if (val) val->data.as_bool = v ? 1 : 0;
    return val;
}

JsonValue* json_int(long v) {
    JsonValue* val = json_new(JSON_INT);
    if (val) val->data.as_int = v;
    return val;
}

JsonValue* json_float(double v) {
    JsonValue* val = json_new(JSON_FLOAT);
    if (val) val->data.as_float = v;
    return val;
}

JsonValue* json_string(BrixString* s) {
    JsonValue* val = json_new(JSON_STRING);
    if (val) {
        val->data.as_string = s ? string_retain(s) : str_new("");
    }
    return val;
}

JsonValue* json_array(void) {
    JsonValue* val = json_new(JSON_ARRAY);
    if (val) {
        JsonArray* arr = (JsonArray*)malloc(sizeof(JsonArray));
        arr->len = 0;
        arr->cap = 4;
        arr->items = (JsonValue**)malloc(sizeof(JsonValue*) * arr->cap);
        val->data.as_array = arr;
    }
    return val;
}

JsonValue* json_object(void) {
    JsonValue* val = json_new(JSON_OBJECT);
    if (val) {
        JsonObject* obj = (JsonObject*)malloc(sizeof(JsonObject));
        obj->head = NULL;
        obj->tail = NULL;
        obj->count = 0;
        val->data.as_object = obj;
    }
    return val;
}

static int json_contains_pointer(JsonValue* container, JsonValue* target, int depth) {
    if (!container || !target || depth > 64) return 1;
    if (container == target) return 1;
    if (container->tag == JSON_ARRAY && container->data.as_array) {
        JsonArray* arr = container->data.as_array;
        for (size_t i = 0; i < arr->len; i++) {
            if (json_contains_pointer(arr->items[i], target, depth + 1)) return 1;
        }
    } else if (container->tag == JSON_OBJECT && container->data.as_object) {
        JsonObjectEntry* curr = container->data.as_object->head;
        while (curr) {
            if (json_contains_pointer(curr->val, target, depth + 1)) return 1;
            curr = curr->next;
        }
    }
    return 0;
}

JsonValue* json_get(JsonValue* obj, BrixString* key) {
    if (!obj || obj->tag != JSON_OBJECT || !obj->data.as_object || !key || !key->data) {
        return NULL;
    }
    JsonObjectEntry* curr = obj->data.as_object->head;
    while (curr) {
        if (curr->key && curr->key->data && strcmp(curr->key->data, key->data) == 0) {
            return json_retain(curr->val);
        }
        curr = curr->next;
    }
    return NULL;
}

JsonValue* json_index(JsonValue* arr, long idx) {
    if (!arr || arr->tag != JSON_ARRAY || !arr->data.as_array || idx < 0) {
        return NULL;
    }
    JsonArray* a = arr->data.as_array;
    if ((size_t)idx >= a->len) {
        return NULL;
    }
    return json_retain(a->items[idx]);
}

void json_set(JsonValue* obj, BrixString* key, JsonValue* val) {
    if (!obj || obj->tag != JSON_OBJECT || !obj->data.as_object || !key || !val) {
        return;
    }
    if (json_contains_pointer(val, obj, 0)) {
        return;
    }
    JsonObject* o = obj->data.as_object;
    JsonObjectEntry* curr = o->head;
    while (curr) {
        if (curr->key && curr->key->data && strcmp(curr->key->data, key->data) == 0) {
            JsonValue* old_val = curr->val;
            curr->val = json_retain(val);
            if (old_val) json_release(old_val);
            return;
        }
        curr = curr->next;
    }
    JsonObjectEntry* entry = (JsonObjectEntry*)malloc(sizeof(JsonObjectEntry));
    entry->key = string_retain(key);
    entry->val = json_retain(val);
    entry->next = NULL;
    if (o->tail) {
        o->tail->next = entry;
        o->tail = entry;
    } else {
        o->head = entry;
        o->tail = entry;
    }
    o->count++;
}

void json_array_push(JsonValue* arr, JsonValue* val) {
    if (!arr || arr->tag != JSON_ARRAY || !arr->data.as_array || !val) {
        return;
    }
    if (json_contains_pointer(val, arr, 0)) {
        return;
    }
    JsonArray* a = arr->data.as_array;
    if (a->len >= a->cap) {
        size_t new_cap = a->cap * 2;
        JsonValue** new_items = (JsonValue**)realloc(a->items, sizeof(JsonValue*) * new_cap);
        if (!new_items) return;
        a->items = new_items;
        a->cap = new_cap;
    }
    a->items[a->len++] = json_retain(val);
}

long json_array_len(JsonValue* arr) {
    if (!arr || arr->tag != JSON_ARRAY || !arr->data.as_array) {
        return 0;
    }
    return (long)arr->data.as_array->len;
}

BrixString* json_as_string(JsonValue* v) {
    if (!v) return NULL;
    if (v->tag == JSON_STRING && v->data.as_string) {
        return string_retain(v->data.as_string);
    }
    return NULL;
}

long json_as_int(JsonValue* v, int* ok) {
    if (!v) { if (ok) *ok = 0; return 0; }
    if (v->tag == JSON_INT) {
        if (ok) *ok = 1;
        return v->data.as_int;
    }
    if (v->tag == JSON_FLOAT) {
        if (ok) *ok = 1;
        return (long)v->data.as_float;
    }
    if (v->tag == JSON_BOOL) {
        if (ok) *ok = 1;
        return v->data.as_bool;
    }
    if (ok) *ok = 0;
    return 0;
}

double json_as_float(JsonValue* v, int* ok) {
    if (!v) { if (ok) *ok = 0; return 0.0; }
    if (v->tag == JSON_FLOAT) {
        if (ok) *ok = 1;
        return v->data.as_float;
    }
    if (v->tag == JSON_INT) {
        if (ok) *ok = 1;
        return (double)v->data.as_int;
    }
    if (ok) *ok = 0;
    return 0.0;
}

long json_as_bool(JsonValue* v, int* ok) {
    if (!v) { if (ok) *ok = 0; return 0; }
    if (v->tag == JSON_BOOL) {
        if (ok) *ok = 1;
        return v->data.as_bool;
    }
    if (ok) *ok = 0;
    return 0;
}

long json_tag(JsonValue* v) {
    if (!v) return (long)JSON_NULL;
    return (long)v->tag;
}

static void json_parse_skip_whitespace(const char** p) {
    while (**p == ' ' || **p == '\t' || **p == '\n' || **p == '\r') {
        (*p)++;
    }
}

static JsonValue* json_parse_value(const char** p, int depth);

static JsonValue* json_parse_string_val(const char** p) {
    if (**p != '"') return NULL;
    (*p)++;
    char buf[4096];
    size_t out_len = 0;
    while (**p && **p != '"') {
        if (out_len + 4 >= sizeof(buf)) return NULL;
        if (**p == '\\') {
            (*p)++;
            if (**p == '"') buf[out_len++] = '"';
            else if (**p == '\\') buf[out_len++] = '\\';
            else if (**p == '/') buf[out_len++] = '/';
            else if (**p == 'b') buf[out_len++] = '\b';
            else if (**p == 'f') buf[out_len++] = '\f';
            else if (**p == 'n') buf[out_len++] = '\n';
            else if (**p == 'r') buf[out_len++] = '\r';
            else if (**p == 't') buf[out_len++] = '\t';
            else if (**p == 'u') {
                (*p)++;
                unsigned int codepoint = 0;
                for (int k = 0; k < 4; k++) {
                    char c = **p;
                    if (!c) return NULL;
                    int hex = 0;
                    if (c >= '0' && c <= '9') hex = c - '0';
                    else if (c >= 'a' && c <= 'f') hex = c - 'a' + 10;
                    else if (c >= 'A' && c <= 'F') hex = c - 'A' + 10;
                    else return NULL;
                    codepoint = (codepoint << 4) | hex;
                    (*p)++;
                }
                (*p)--;
                if (codepoint == 0) return NULL;
                if (codepoint <= 0x7F) {
                    buf[out_len++] = (char)codepoint;
                } else if (codepoint <= 0x7FF) {
                    buf[out_len++] = (char)(0xC0 | (codepoint >> 6));
                    buf[out_len++] = (char)(0x80 | (codepoint & 0x3F));
                } else {
                    buf[out_len++] = (char)(0xE0 | (codepoint >> 12));
                    buf[out_len++] = (char)(0x80 | ((codepoint >> 6) & 0x3F));
                    buf[out_len++] = (char)(0x80 | (codepoint & 0x3F));
                }
            } else return NULL;
        } else {
            buf[out_len++] = **p;
        }
        (*p)++;
    }
    if (**p != '"') return NULL;
    (*p)++;
    buf[out_len] = '\0';
    BrixString* s = str_new(buf);
    JsonValue* v = json_string(s);
    string_release(s);
    return v;
}

static JsonValue* json_parse_number_val(const char** p) {
    const char* start = *p;
    if (**p == '-') (*p)++;
    if (!isdigit((unsigned char)**p)) return NULL;
    while (isdigit((unsigned char)**p)) (*p)++;
    int is_float = 0;
    if (**p == '.') {
        is_float = 1;
        (*p)++;
        if (!isdigit((unsigned char)**p)) return NULL;
        while (isdigit((unsigned char)**p)) (*p)++;
    }
    if (**p == 'e' || **p == 'E') {
        is_float = 1;
        (*p)++;
        if (**p == '+' || **p == '-') (*p)++;
        if (!isdigit((unsigned char)**p)) return NULL;
        while (isdigit((unsigned char)**p)) (*p)++;
    }
    char num_buf[128];
    size_t num_len = *p - start;
    if (num_len >= sizeof(num_buf)) return NULL;
    memcpy(num_buf, start, num_len);
    num_buf[num_len] = '\0';

    if (is_float) {
        double d = strtod(num_buf, NULL);
        return json_float(d);
    } else {
        long l = strtol(num_buf, NULL, 10);
        return json_int(l);
    }
}

static JsonValue* json_parse_array_val(const char** p, int depth) {
    if (**p != '[') return NULL;
    (*p)++;
    JsonValue* arr = json_array();
    json_parse_skip_whitespace(p);
    if (**p == ']') {
        (*p)++;
        return arr;
    }
    while (**p) {
        JsonValue* item = json_parse_value(p, depth + 1);
        if (!item) {
            json_release(arr);
            return NULL;
        }
        json_array_push(arr, item);
        json_release(item);
        json_parse_skip_whitespace(p);
        if (**p == ']') {
            (*p)++;
            return arr;
        }
        if (**p != ',') {
            json_release(arr);
            return NULL;
        }
        (*p)++;
        json_parse_skip_whitespace(p);
    }
    json_release(arr);
    return NULL;
}

static JsonValue* json_parse_object_val(const char** p, int depth) {
    if (**p != '{') return NULL;
    (*p)++;
    JsonValue* obj = json_object();
    json_parse_skip_whitespace(p);
    if (**p == '}') {
        (*p)++;
        return obj;
    }
    while (**p) {
        json_parse_skip_whitespace(p);
        if (**p != '"') {
            json_release(obj);
            return NULL;
        }
        JsonValue* key_val = json_parse_string_val(p);
        if (!key_val || key_val->tag != JSON_STRING) {
            if (key_val) json_release(key_val);
            json_release(obj);
            return NULL;
        }
        BrixString* key_str = key_val->data.as_string;
        json_parse_skip_whitespace(p);
        if (**p != ':') {
            json_release(key_val);
            json_release(obj);
            return NULL;
        }
        (*p)++;
        json_parse_skip_whitespace(p);
        JsonValue* val = json_parse_value(p, depth + 1);
        if (!val) {
            json_release(key_val);
            json_release(obj);
            return NULL;
        }
        json_set(obj, key_str, val);
        json_release(key_val);
        json_release(val);
        json_parse_skip_whitespace(p);
        if (**p == '}') {
            (*p)++;
            return obj;
        }
        if (**p != ',') {
            json_release(obj);
            return NULL;
        }
        (*p)++;
        json_parse_skip_whitespace(p);
    }
    json_release(obj);
    return NULL;
}

static JsonValue* json_parse_value(const char** p, int depth) {
    if (depth > 64) return NULL;
    json_parse_skip_whitespace(p);
    if (!**p) return NULL;
    if (strncmp(*p, "null", 4) == 0) {
        *p += 4;
        return json_null();
    }
    if (strncmp(*p, "true", 4) == 0) {
        *p += 4;
        return json_bool(1);
    }
    if (strncmp(*p, "false", 5) == 0) {
        *p += 5;
        return json_bool(0);
    }
    if (**p == '"') return json_parse_string_val(p);
    if (**p == '[') return json_parse_array_val(p, depth);
    if (**p == '{') return json_parse_object_val(p, depth);
    if (**p == '-' || isdigit((unsigned char)**p)) return json_parse_number_val(p);
    return NULL;
}

JsonValue* json_parse(BrixString* input) {
    if (!input || !input->data) return NULL;
    const char* p = input->data;
    JsonValue* v = json_parse_value(&p, 0);
    if (!v) return NULL;
    json_parse_skip_whitespace(&p);
    if (*p != '\0') {
        json_release(v);
        return NULL;
    }
    return v;
}

#define JSON_BUF_APPEND_STR(str) do { \
    size_t slen = strlen(str); \
    while (*len + slen + 1 >= *cap) { *cap *= 2; *buf = (char*)realloc(*buf, *cap); } \
    memcpy(*buf + *len, str, slen); \
    *len += slen; \
    (*buf)[*len] = '\0'; \
} while(0)

#define JSON_BUF_APPEND_CHAR(c) do { \
    while (*len + 2 >= *cap) { *cap *= 2; *buf = (char*)realloc(*buf, *cap); } \
    (*buf)[(*len)++] = (c); \
    (*buf)[*len] = '\0'; \
} while(0)

static void json_append_escaped_str(const char* s, char** buf, size_t* len, size_t* cap) {
    if (!s) s = "";
    while (*s) {
        unsigned char uc = (unsigned char)*s;
        if (uc == '"') JSON_BUF_APPEND_STR("\\\"");
        else if (uc == '\\') JSON_BUF_APPEND_STR("\\\\");
        else if (uc == '\b') JSON_BUF_APPEND_STR("\\b");
        else if (uc == '\f') JSON_BUF_APPEND_STR("\\f");
        else if (uc == '\n') JSON_BUF_APPEND_STR("\\n");
        else if (uc == '\r') JSON_BUF_APPEND_STR("\\r");
        else if (uc == '\t') JSON_BUF_APPEND_STR("\\t");
        else if (uc < 0x20) {
            char hex[8];
            snprintf(hex, sizeof(hex), "\\u00%02x", (unsigned int)uc);
            JSON_BUF_APPEND_STR(hex);
        } else {
            JSON_BUF_APPEND_CHAR(*s);
        }
        s++;
    }
}

static void json_stringify_helper(JsonValue* v, char** buf, size_t* len, size_t* cap, int pretty, int indent_spaces, int indent_level) {
    if (!v) {
        JSON_BUF_APPEND_STR("null");
        return;
    }

    switch (v->tag) {
        case JSON_NULL:
            JSON_BUF_APPEND_STR("null");
            break;
        case JSON_BOOL:
            JSON_BUF_APPEND_STR(v->data.as_bool ? "true" : "false");
            break;
        case JSON_INT: {
            char num[64];
            snprintf(num, sizeof(num), "%ld", v->data.as_int);
            JSON_BUF_APPEND_STR(num);
            break;
        }
        case JSON_FLOAT: {
            char num[64];
            snprintf(num, sizeof(num), "%.15g", v->data.as_float);
            JSON_BUF_APPEND_STR(num);
            break;
        }
        case JSON_STRING: {
            JSON_BUF_APPEND_CHAR('"');
            const char* s = v->data.as_string ? v->data.as_string->data : "";
            json_append_escaped_str(s, buf, len, cap);
            JSON_BUF_APPEND_CHAR('"');
            break;
        }
        case JSON_ARRAY: {
            JSON_BUF_APPEND_CHAR('[');
            JsonArray* arr = v->data.as_array;
            if (arr && arr->len > 0) {
                if (pretty) {
                    JSON_BUF_APPEND_CHAR('\n');
                }
                for (size_t i = 0; i < arr->len; i++) {
                    if (pretty) {
                        for (int k = 0; k < (indent_level + 1) * indent_spaces; k++) JSON_BUF_APPEND_CHAR(' ');
                    }
                    json_stringify_helper(arr->items[i], buf, len, cap, pretty, indent_spaces, indent_level + 1);
                    if (i + 1 < arr->len) JSON_BUF_APPEND_CHAR(',');
                    if (pretty) JSON_BUF_APPEND_CHAR('\n');
                }
                if (pretty) {
                    for (int k = 0; k < indent_level * indent_spaces; k++) JSON_BUF_APPEND_CHAR(' ');
                }
            }
            JSON_BUF_APPEND_CHAR(']');
            break;
        }
        case JSON_OBJECT: {
            JSON_BUF_APPEND_CHAR('{');
            JsonObject* obj = v->data.as_object;
            if (obj && obj->head) {
                if (pretty) JSON_BUF_APPEND_CHAR('\n');
                JsonObjectEntry* curr = obj->head;
                while (curr) {
                    if (pretty) {
                        for (int k = 0; k < (indent_level + 1) * indent_spaces; k++) JSON_BUF_APPEND_CHAR(' ');
                    }
                    JSON_BUF_APPEND_CHAR('"');
                    const char* ks = curr->key ? curr->key->data : "";
                    json_append_escaped_str(ks, buf, len, cap);
                    JSON_BUF_APPEND_STR(pretty ? "\": " : "\":");
                    json_stringify_helper(curr->val, buf, len, cap, pretty, indent_spaces, indent_level + 1);
                    if (curr->next) JSON_BUF_APPEND_CHAR(',');
                    if (pretty) JSON_BUF_APPEND_CHAR('\n');
                    curr = curr->next;
                }
                if (pretty) {
                    for (int k = 0; k < indent_level * indent_spaces; k++) JSON_BUF_APPEND_CHAR(' ');
                }
            }
            JSON_BUF_APPEND_CHAR('}');
            break;
        }
    }
}

BrixString* json_stringify(JsonValue* val) {
    size_t cap = 256;
    size_t len = 0;
    char* buf = (char*)malloc(cap);
    buf[0] = '\0';
    json_stringify_helper(val, &buf, &len, &cap, 0, 2, 0);
    BrixString* res = str_new(buf);
    free(buf);
    return res;
}

BrixString* json_stringify_pretty(JsonValue* val, long indent) {
    int indent_spaces = (int)indent;
    if (indent_spaces <= 0) indent_spaces = 2;
    size_t cap = 256;
    size_t len = 0;
    char* buf = (char*)malloc(cap);
    buf[0] = '\0';
    json_stringify_helper(val, &buf, &len, &cap, 1, indent_spaces, 0);
    BrixString* res = str_new(buf);
    free(buf);
    return res;
}


// ==========================================
// SECTION 2.10: EMBEDDING<DIM> (v2.0 Grupo A Fase 1)
// ==========================================
//
// BrixEmbedding is a fixed-dimension f64 vector — double, not float, to match
// Brix's `float` (f64) and Matrix's existing `double*` storage, so a
// round-trip through .to_matrix()/Embedding<DIM>(matrix) never loses
// precision. `dim` is kept on the struct and checked defensively even though
// the type checker already enforces matching dimensions at compile time
// (both Embedding<D> values carry D in their own BrixType) — this guards
// only against an internal compiler bug reaching runtime, not a user-facing
// error path.
//
// Similarity methods via BLAS added in Fase 2 (below); EmbeddingBatch in
// SECTION 2.11.

typedef struct {
  long ref_count;
  long dim;
  double *data;
} BrixEmbedding;

// Construct from an existing Matrix* (already double*-backed) — used both for
// a float array literal (which already compiles to a Matrix) and for
// `Embedding<DIM>(some_matrix_variable)`. Requires shape exactly 1×dim;
// anything else is a runtime abort, never a compile-time error, since
// Matrix's shape (rows/cols) lives on the struct, not in BrixType. Copies
// `m`'s data — the caller is still responsible for releasing `m` itself
// (this never takes ownership of `m`).
BrixEmbedding *brix_embedding_from_matrix(Matrix *m, long dim) {
  if (!m) {
    fprintf(stderr, "Error: brix_embedding_from_matrix called with NULL\n");
    exit(1);
  }
  if (m->rows != 1 || m->cols != dim) {
    fprintf(stderr,
            "Error: Embedding<%ld> construction requires a 1x%ld Matrix, got "
            "%ldx%ld\n",
            dim, dim, m->rows, m->cols);
    exit(1);
  }

  BrixEmbedding *e = (BrixEmbedding *)malloc(sizeof(BrixEmbedding));
  e->ref_count = 1;
  e->dim = dim;
  e->data = (double *)malloc(dim * sizeof(double));
  memcpy(e->data, m->data, dim * sizeof(double));
  return e;
}

// Copy out to a new, independent Matrix (1xdim) — releasing the Embedding
// afterward never invalidates the returned Matrix, and vice versa.
Matrix *brix_embedding_to_matrix(BrixEmbedding *e) {
  if (!e) {
    fprintf(stderr, "Error: brix_embedding_to_matrix called with NULL\n");
    exit(1);
  }
  Matrix *m = matrix_new(1, e->dim);
  memcpy(m->data, e->data, e->dim * sizeof(double));
  return m;
}

void *brix_embedding_retain(BrixEmbedding *e) {
  if (!e) return NULL;
  e->ref_count++;
  return e;
}

void brix_embedding_release(BrixEmbedding *e) {
  if (!e) return;
  e->ref_count--;
  if (e->ref_count == 0) {
    free(e->data);
    free(e);
  }
}

// --- Similarity methods (v2.0 Grupo A Fase 2) ---
//
// BLAS Fortran-convention symbols (trailing `_`), the same convention the
// existing LAPACK wrappers use (dgetrf_ etc.) and resolved by the
// `-llapack -lblas` link flags the driver already passes. Every argument is
// passed by pointer, Fortran-style.
extern double ddot_(int *n, double *x, int *incx, double *y, int *incy);
extern double dnrm2_(int *n, double *x, int *incx);

// Defensive invariant check, never a user-facing error path: two
// Embedding<D> operands with different D are rejected at compile time
// (E102), so a mismatch here means a compiler bug. BLAS takes `int` lengths,
// while dim comes from a u32 const generic, so dims above INT_MAX are
// rejected too instead of silently truncating.
static int brix_embedding_check_pair(BrixEmbedding *a, BrixEmbedding *b,
                                     const char *op) {
  if (!a || !b) {
    fprintf(stderr, "Error: Embedding.%s called with NULL (internal compiler bug)\n", op);
    exit(1);
  }
  if (a->dim != b->dim) {
    fprintf(stderr,
            "Error: Embedding.%s dimension mismatch %ld vs %ld (internal "
            "compiler bug — should have been rejected at compile time)\n",
            op, a->dim, b->dim);
    exit(1);
  }
  if (a->dim > INT_MAX) {
    fprintf(stderr, "Error: Embedding.%s dimension %ld exceeds BLAS int range\n",
            op, a->dim);
    exit(1);
  }
  return (int)a->dim;
}

double brix_embedding_dot(BrixEmbedding *a, BrixEmbedding *b) {
  int n = brix_embedding_check_pair(a, b, "dot_product");
  int one = 1;
  return ddot_(&n, a->data, &one, b->data, &one);
}

double brix_embedding_euclidean(BrixEmbedding *a, BrixEmbedding *b) {
  int n = brix_embedding_check_pair(a, b, "euclidean_distance");
  int one = 1;
  double *diff = (double *)malloc((size_t)n * sizeof(double));
  for (int i = 0; i < n; i++) diff[i] = a->data[i] - b->data[i];
  double d = dnrm2_(&n, diff, &one);
  free(diff);
  return d;
}

// Zero-vector contract: if either norm is 0 the ratio is 0/0 — returns 0.0
// (never NaN/Inf), an explicit roadmap decision.
double brix_embedding_cosine(BrixEmbedding *a, BrixEmbedding *b) {
  int n = brix_embedding_check_pair(a, b, "cosine_similarity");
  int one = 1;
  double na = dnrm2_(&n, a->data, &one);
  double nb = dnrm2_(&n, b->data, &one);
  if (na == 0.0 || nb == 0.0) return 0.0;
  return ddot_(&n, a->data, &one, b->data, &one) / (na * nb);
}

// ==========================================
// SECTION 2.11: EMBEDDINGBATCH<DIM> (v2.0 Grupo A Fase 3)
// ==========================================
//
// Growable array of BrixEmbedding* with shared ownership: add() retains,
// get() retains for the caller, release() releases every element. All
// embeddings share the batch's dim (enforced at compile time — E102 — and
// re-checked here only as an internal-invariant defense).

typedef struct {
  long ref_count;
  long dim;
  long len;
  long cap;
  BrixEmbedding **items;
} BrixEmbeddingBatch;

BrixEmbeddingBatch *brix_embedding_batch_new(long dim, long cap) {
  if (cap < 0) {
    fprintf(stderr, "Error: EmbeddingBatch capacity must be >= 0, got %ld\n", cap);
    exit(1);
  }
  if ((size_t)cap > SIZE_MAX / sizeof(BrixEmbedding *)) {
    fprintf(stderr, "Error: EmbeddingBatch capacity overflow (%ld)\n", cap);
    exit(1);
  }
  BrixEmbeddingBatch *b = (BrixEmbeddingBatch *)malloc(sizeof(BrixEmbeddingBatch));
  BrixEmbedding **items =
      cap > 0 ? (BrixEmbedding **)malloc((size_t)cap * sizeof(BrixEmbedding *)) : NULL;
  if (!b || (cap > 0 && !items)) {
    fprintf(stderr, "Error: EmbeddingBatch out of memory (capacity %ld)\n", cap);
    exit(1);
  }
  b->ref_count = 1;
  b->dim = dim;
  b->len = 0;
  b->cap = cap;
  b->items = items;
  return b;
}

void brix_embedding_batch_add(BrixEmbeddingBatch *b, BrixEmbedding *e) {
  if (!b || !e || e->dim != b->dim) {
    fprintf(stderr, "Error: EmbeddingBatch.add invalid operand (internal compiler bug)\n");
    exit(1);
  }
  if (b->len == b->cap) {
    // Check before multiplying: overflowing a signed long is UB.
    if (b->cap > LONG_MAX / 2) {
      fprintf(stderr, "Error: EmbeddingBatch capacity overflow\n");
      exit(1);
    }
    long new_cap = b->cap == 0 ? 4 : b->cap * 2;
    if ((size_t)new_cap > SIZE_MAX / sizeof(BrixEmbedding *)) {
      fprintf(stderr, "Error: EmbeddingBatch capacity overflow\n");
      exit(1);
    }
    BrixEmbedding **items =
        (BrixEmbedding **)realloc(b->items, (size_t)new_cap * sizeof(BrixEmbedding *));
    if (!items) {
      fprintf(stderr, "Error: EmbeddingBatch out of memory\n");
      exit(1);
    }
    b->items = items;
    b->cap = new_cap;
  }
  brix_embedding_retain(e);
  b->items[b->len++] = e;
}

// Returns an owned reference (retained for the caller).
BrixEmbedding *brix_embedding_batch_get(BrixEmbeddingBatch *b, long i) {
  if (i < 0 || i >= b->len) {
    fprintf(stderr, "Error: EmbeddingBatch.get(%ld) out of bounds (len %ld)\n", i, b->len);
    exit(1);
  }
  return (BrixEmbedding *)brix_embedding_retain(b->items[i]);
}

long brix_embedding_batch_len(BrixEmbeddingBatch *b) { return b->len; }

void *brix_embedding_batch_retain(BrixEmbeddingBatch *b) {
  if (!b) return NULL;
  b->ref_count++;
  return b;
}

void brix_embedding_batch_release(BrixEmbeddingBatch *b) {
  if (!b) return;
  b->ref_count--;
  if (b->ref_count == 0) {
    for (long i = 0; i < b->len; i++) brix_embedding_release(b->items[i]);
    free(b->items);
    free(b);
  }
}

// --- find_nearest: top-k by cosine similarity ---

typedef struct {
  double score;
  long idx;
} BrixNearest;

// "a ranks before b": higher score first; ties broken by lower index, so the
// result is deterministic.
static int brix_nearest_before(BrixNearest a, BrixNearest b) {
  return a.score > b.score || (a.score == b.score && a.idx < b.idx);
}

static int brix_nearest_cmp(const void *pa, const void *pb) {
  BrixNearest a = *(const BrixNearest *)pa, b = *(const BrixNearest *)pb;
  return brix_nearest_before(a, b) ? -1 : (brix_nearest_before(b, a) ? 1 : 0);
}

// Min-heap ordered so the root is the *worst* kept candidate.
static void brix_nearest_sift_down(BrixNearest *h, long n, long i) {
  for (;;) {
    long worst = i, l = 2 * i + 1, r = 2 * i + 2;
    if (l < n && brix_nearest_before(h[worst], h[l])) worst = l;
    if (r < n && brix_nearest_before(h[worst], h[r])) worst = r;
    if (worst == i) return;
    BrixNearest t = h[i];
    h[i] = h[worst];
    h[worst] = t;
    i = worst;
  }
}

static void brix_nearest_sift_up(BrixNearest *h, long i) {
  while (i > 0) {
    long p = (i - 1) / 2;
    if (!brix_nearest_before(h[p], h[i])) return;
    BrixNearest t = h[i];
    h[i] = h[p];
    h[p] = t;
    i = p;
  }
}

// Returns a 1×min(k, len) IntMatrix of batch indices, most similar first.
// O(n·DIM + n·log k): one cosine per element, a size-k heap of the best.
IntMatrix *brix_embedding_batch_find_nearest(BrixEmbeddingBatch *b, BrixEmbedding *q, long k) {
  if (k < 0) {
    fprintf(stderr, "Error: EmbeddingBatch.find_nearest k must be >= 0, got %ld\n", k);
    exit(1);
  }
  long count = k < b->len ? k : b->len;
  IntMatrix *out = intmatrix_new(1, count);
  if (count == 0) return out;

  BrixNearest *heap = (BrixNearest *)malloc((size_t)count * sizeof(BrixNearest));
  if (!heap) {
    fprintf(stderr, "Error: EmbeddingBatch.find_nearest out of memory (k %ld)\n", count);
    exit(1);
  }
  long n = 0;
  for (long i = 0; i < b->len; i++) {
    BrixNearest c = {brix_embedding_cosine(b->items[i], q), i};
    if (n < count) {
      heap[n] = c;
      brix_nearest_sift_up(heap, n++);
    } else if (brix_nearest_before(c, heap[0])) {
      heap[0] = c;
      brix_nearest_sift_down(heap, n, 0);
    }
  }
  qsort(heap, (size_t)count, sizeof(BrixNearest), brix_nearest_cmp);
  for (long i = 0; i < count; i++) out->data[i] = heap[i].idx;
  free(heap);
  return out;
}

// ==========================================
// SECTION 2.12: VECTORDB (v2.1 Grupo A Fases 1-5, Grupo B)
// ==========================================
//
// A BrixVectorDB is one connection to one Pinecone index (its data-plane
// host), one Chroma collection (Grupo B Fase 1), one Weaviate class (Grupo B
// Fase 2) or one Milvus collection (Grupo B Fase 3); see their parts at the
// end of this section. connect() validates the config locally and does no
// network I/O.
// Each operation is one HTTPS request through the connection's own CURL easy
// handle; reusing it lets libcurl keep the TCP/TLS connection alive between
// calls. Network and API failures never abort: a read returns NULL (nil in
// Brix), a write returns 0, and the reason is kept in last_error. Invalid config
// (empty host/key, plain http:// to a non-loopback host, control characters)
// aborts in connect(). The API key is only ever sent in the Api-Key header;
// it is never part of last_error or any message.

#define BRIX_PINECONE_API_VERSION "2025-01"
#define BRIX_VDB_MAX_RESPONSE (64L * 1024 * 1024)

#define BRIX_VDB_PINECONE 0
#define BRIX_VDB_CHROMA 1
#define BRIX_VDB_WEAVIATE 2
#define BRIX_VDB_MILVUS 3

typedef struct {
  long ref_count;
  long provider;    // BRIX_VDB_PINECONE / _CHROMA / _WEAVIATE / _MILVUS
  char *base_url;   // "https://host[:port]", no trailing '/'
  char *api_key;    // Chroma: "" = no auth
  char *ns;         // Pinecone: namespace ("" = default); Chroma and
                    // Milvus: collection; Weaviate: class
  char *db_path;    // Chroma: "/api/v2/tenants/<t>/databases/<d>";
                    // Milvus: the database (NULL = default)
  char *coll_path;  // Resolved on first use, NULL until then. Chroma:
                    // db_path + "/collections/<uuid>"; Weaviate: the
                    // class name as the schema spells it; Milvus: the
                    // primary key field
  char *vec_field;  // Milvus: the vector field (with coll_path)
  CURL *curl;
  char *last_error; // NULL = the last operation succeeded
  long timeout_ms;  // per request attempt (set_timeout)
  long retries;     // extra attempts on 429/502/503/504 (set_retries)
  long last_status; // HTTP status of the last response, 0 = none
} BrixVectorDB;

#define BRIX_VDB_DEFAULT_TIMEOUT_MS 30000L
#define BRIX_VDB_DEFAULT_RETRIES 3L
#define BRIX_VDB_MAX_RETRIES 10L
#define BRIX_VDB_BACKOFF_BASE_MS 100L
#define BRIX_VDB_MAX_DELAY_MS 30000L

// env(name) -> string? (NULL when the variable is not set).
BrixString *brix_env(BrixString *name) {
  if (!name) return NULL;
  const char *v = getenv(name->data);
  return v ? str_new((char *)v) : NULL;
}

static void vdb_config_error(const char *msg) {
  fprintf(stderr, "Error: vectordb.connect: %s\n", msg);
  exit(1);
}

static char *vdb_strdup(const char *s) {
  char *d = strdup(s);
  if (!d) vdb_config_error("out of memory");
  return d;
}

// Control characters would allow header injection through the key, or
// malformed URLs through the host.
static int vdb_has_ctl(const char *s) {
  for (; *s; s++) {
    if ((unsigned char)*s < 0x20 || *s == 0x7f) return 1;
  }
  return 0;
}

static int vdb_is_loopback(const char *host) {
  return strcmp(host, "localhost") == 0 || strcmp(host, "127.0.0.1") == 0 ||
         strcmp(host, "[::1]") == 0;
}

// Parse `base` with libcurl's own URL parser (the one the request will use),
// so the checks see exactly the host curl will connect to: "localhost:1@evil"
// is user "localhost", host "evil".
static void vdb_check_url(const char *base, int plain_http) {
  CURLU *u = curl_url();
  if (!u) vdb_config_error("out of memory");
  if (curl_url_set(u, CURLUPART_URL, base, 0) != CURLUE_OK) {
    vdb_config_error("host is not a valid URL");
  }
  char *part = NULL;
  if (curl_url_get(u, CURLUPART_USER, &part, 0) == CURLUE_OK ||
      curl_url_get(u, CURLUPART_PASSWORD, &part, 0) == CURLUE_OK) {
    vdb_config_error("host must not contain credentials (user@host)");
  }
  if (curl_url_get(u, CURLUPART_QUERY, &part, 0) == CURLUE_OK ||
      curl_url_get(u, CURLUPART_FRAGMENT, &part, 0) == CURLUE_OK) {
    vdb_config_error("host must not contain a query or fragment");
  }
  if (curl_url_get(u, CURLUPART_HOST, &part, 0) != CURLUE_OK || !*part) {
    vdb_config_error("host is empty");
  }
  if (plain_http && !vdb_is_loopback(part)) {
    vdb_config_error("plain http:// is only allowed for localhost, "
                     "127.0.0.1 and [::1]; use https://");
  }
  curl_free(part);
  curl_url_cleanup(u);
}

// 1 when `s` is a GraphQL / Weaviate name: [_A-Za-z][_0-9A-Za-z]*.
// ASCII only (no locale): this check keeps names from breaking out of
// GraphQL text.
static int vdb_is_name(const char *s) {
  for (const char *p = s; *p; p++) {
    int alpha = (*p >= 'A' && *p <= 'Z') || (*p >= 'a' && *p <= 'z') || *p == '_';
    if (!alpha && (p == s || *p < '0' || *p > '9')) return 0;
  }
  return *s != '\0';
}

// Chroma: "/api/v2/tenants/<t>/databases/<d>" for `spec` ("name" uses the
// default tenant and database, "tenant/database/name" names them). Sets
// *name to the collection name (owned).
static char *chroma_db_path(const char *spec, char **name) {
  const char *s1 = strchr(spec, '/');
  const char *s2 = s1 ? strchr(s1 + 1, '/') : NULL;
  char *tenant, *database;
  if (!s1) {
    tenant = vdb_strdup("default_tenant");
    database = vdb_strdup("default_database");
    *name = vdb_strdup(spec);
  } else if (s2 && !strchr(s2 + 1, '/') && s1 > spec && s2 > s1 + 1 && s2[1]) {
    tenant = strndup(spec, s1 - spec);
    database = strndup(s1 + 1, s2 - s1 - 1);
    *name = vdb_strdup(s2 + 1);
    if (!tenant || !database) vdb_config_error("out of memory");
  } else {
    vdb_config_error("the collection must be \"name\" or "
                     "\"tenant/database/name\"");
    return NULL;
  }
  // "." / ".." would become dot segments that curl normalizes away.
  const char *parts[] = {tenant, database, *name};
  for (int i = 0; i < 3; i++) {
    if (strcmp(parts[i], ".") == 0 || strcmp(parts[i], "..") == 0) {
      vdb_config_error("tenant, database and collection must not be "
                       "\".\" or \"..\"");
    }
  }
  char *t = curl_easy_escape(NULL, tenant, 0);
  char *d = curl_easy_escape(NULL, database, 0);
  if (!t || !d) vdb_config_error("out of memory");
  size_t len = strlen(t) + strlen(d) + 32;
  char *path = malloc(len);
  if (!path) vdb_config_error("out of memory");
  snprintf(path, len, "/api/v2/tenants/%s/databases/%s", t, d);
  curl_free(t);
  curl_free(d);
  free(tenant);
  free(database);
  return path;
}

// vectordb.connect(provider, host, api_key[, ns]). Pinecone: ns is the
// namespace (NULL = default). Chroma / Weaviate / Milvus: ns is the
// collection or class (required, checked at compile time) and an empty
// api_key means no auth.
BrixVectorDB *brix_vectordb_connect(long provider, BrixString *host,
                                    BrixString *api_key, BrixString *ns) {
  const char *h = host ? host->data : "";
  const char *k = api_key ? api_key->data : "";
  const char *n = ns ? ns->data : "";
  if (!*h) vdb_config_error("host is empty");
  if (!*k && provider == BRIX_VDB_PINECONE) vdb_config_error("api_key is empty");
  if (!*n && provider != BRIX_VDB_PINECONE) vdb_config_error("collection is empty");
  if (vdb_has_ctl(h) || strchr(h, ' ')) {
    vdb_config_error("host contains whitespace or control characters");
  }
  if (vdb_has_ctl(k)) vdb_config_error("api_key contains control characters");
  if (vdb_has_ctl(n)) {
    vdb_config_error(provider == BRIX_VDB_PINECONE
                         ? "namespace contains control characters"
                         : "collection contains control characters");
  }
  // The class name goes into URL paths and GraphQL queries verbatim.
  if (provider == BRIX_VDB_WEAVIATE && !vdb_is_name(n)) {
    vdb_config_error("the Weaviate class must be a name: a letter, then "
                     "letters, digits or '_'");
  }
  // Milvus: "name" or "database/name", both names (as Milvus requires).
  const char *slash = strchr(n, '/');
  char *mv_db = NULL;
  if (provider == BRIX_VDB_MILVUS) {
    if (slash) {
      mv_db = strndup(n, slash - n);
      if (!mv_db) vdb_config_error("out of memory");
      n = slash + 1;
    }
    if (!vdb_is_name(n) || (mv_db && !vdb_is_name(mv_db))) {
      vdb_config_error("the Milvus collection must be \"name\" or "
                       "\"database/name\", each a letter or '_', then "
                       "letters, digits or '_'");
    }
  }

  char *base;
  size_t scheme_len;
  if (strncmp(h, "https://", 8) == 0) {
    base = vdb_strdup(h);
    scheme_len = 8;
  } else if (strncmp(h, "http://", 7) == 0) {
    base = vdb_strdup(h);
    scheme_len = 7;
  } else if (strstr(h, "://")) {
    vdb_config_error("unsupported URL scheme (use https://)");
    return NULL;
  } else {
    base = malloc(strlen(h) + 9);
    if (!base) vdb_config_error("out of memory");
    sprintf(base, "https://%s", h);
    scheme_len = 8;
  }
  size_t len = strlen(base);
  while (len > scheme_len && base[len - 1] == '/') base[--len] = '\0';
  if (len == scheme_len) vdb_config_error("host is empty");
  vdb_check_url(base, scheme_len == 7);

  BrixVectorDB *db = malloc(sizeof(BrixVectorDB));
  if (!db) vdb_config_error("out of memory");
  db->curl = curl_easy_init();
  if (!db->curl) vdb_config_error("failed to initialize libcurl");
  db->ref_count = 1;
  db->provider = provider;
  db->base_url = base;
  db->api_key = vdb_strdup(k);
  db->db_path = NULL;
  db->coll_path = NULL;
  db->vec_field = NULL;
  if (provider == BRIX_VDB_CHROMA) {
    db->db_path = chroma_db_path(n, &db->ns);
  } else {
    db->ns = vdb_strdup(n);
    db->db_path = mv_db;
  }
  db->last_error = NULL;
  db->timeout_ms = BRIX_VDB_DEFAULT_TIMEOUT_MS;
  db->retries = BRIX_VDB_DEFAULT_RETRIES;
  db->last_status = 0;
  return db;
}

BrixVectorDB *brix_vectordb_retain(BrixVectorDB *db) {
  if (db) db->ref_count++;
  return db;
}

void brix_vectordb_release(BrixVectorDB *db) {
  if (!db || --db->ref_count > 0) return;
  curl_easy_cleanup(db->curl);
  memset(db->api_key, 0, strlen(db->api_key));
  free(db->api_key);
  free(db->base_url);
  free(db->ns);
  free(db->db_path);
  free(db->coll_path);
  free(db->vec_field);
  free(db->last_error);
  free(db);
}

// Start of every public operation (and of each request): no error, no
// status yet.
static void vdb_begin(BrixVectorDB *db) {
  free(db->last_error);
  db->last_error = NULL;
  db->last_status = 0;
}

static void vdb_set_error(BrixVectorDB *db, const char *fmt, ...) {
  char msg[512];
  va_list ap;
  va_start(ap, fmt);
  vsnprintf(msg, sizeof(msg), fmt, ap);
  va_end(ap);
  free(db->last_error);
  db->last_error = strdup(msg); // NULL on OOM just means "no message"
}

typedef struct {
  char *data;
  size_t len;
  size_t cap;
  int too_large;
} VdbBuf;

static size_t vdb_write_cb(char *ptr, size_t size, size_t nmemb, void *userdata) {
  VdbBuf *b = userdata;
  size_t n = size * nmemb;
  if (b->len + n + 1 > (size_t)BRIX_VDB_MAX_RESPONSE) {
    b->too_large = 1;
    return 0; // aborts the transfer
  }
  if (b->len + n + 1 > b->cap) {
    size_t cap = b->cap ? b->cap : 4096;
    while (cap < b->len + n + 1) cap *= 2;
    char *p = realloc(b->data, cap);
    if (!p) return 0;
    b->data = p;
    b->cap = cap;
  }
  memcpy(b->data + b->len, ptr, n);
  b->len += n;
  b->data[b->len] = '\0';
  return n;
}

static JsonValue *vdb_field(JsonValue *obj, const char *key);

// Send one request to base_url + `prefix` + `path`: a POST with `body` (JSON
// text), or a GET when `body` is NULL; `method` (NULL = that default)
// overrides it, e.g. a DELETE with a body. `query` (already URL-encoded, or
// NULL) is appended after '?'. Returns the parsed JSON response on a 2xx
// status; otherwise NULL with db->last_error set. Errors name `path` only,
// never the prefix (tenant, collection id) or the query string.
static JsonValue *vdb_request_at(BrixVectorDB *db, const char *method,
                                 const char *prefix, const char *path,
                                 const char *query, const char *body) {
  vdb_begin(db);

  size_t url_len = strlen(db->base_url) + strlen(prefix) + strlen(path) +
                   (query ? strlen(query) + 1 : 0) + 1;
  // Pinecone: "Api-Key: k". Chroma: "x-chroma-token: k" (Chroma Cloud) and
  // "Authorization: Bearer k" (self-hosted token auth), both sent. Weaviate:
  // "Authorization: Bearer k", Milvus too ("user:password" or an API key).
  // An empty Chroma/Weaviate/Milvus key sends neither.
  size_t key_len = strlen(db->api_key) + sizeof("Authorization: Bearer ");
  char *url = malloc(url_len);
  char *key_header = malloc(key_len);
  char *bearer = malloc(key_len);
  if (!url || !key_header || !bearer) {
    free(url);
    free(key_header);
    free(bearer);
    vdb_set_error(db, "out of memory");
    return NULL;
  }
  snprintf(url, url_len, "%s%s%s%s%s", db->base_url, prefix, path,
           query ? "?" : "", query ? query : "");

  struct curl_slist *headers = NULL;
  if (db->provider == BRIX_VDB_CHROMA) {
    snprintf(key_header, key_len, "x-chroma-token: %s", db->api_key);
    snprintf(bearer, key_len, "Authorization: Bearer %s", db->api_key);
    if (*db->api_key) {
      headers = curl_slist_append(headers, key_header);
      headers = curl_slist_append(headers, bearer);
    }
  } else if (db->provider == BRIX_VDB_WEAVIATE ||
             db->provider == BRIX_VDB_MILVUS) {
    key_header[0] = '\0';
    snprintf(bearer, key_len, "Authorization: Bearer %s", db->api_key);
    if (*db->api_key) headers = curl_slist_append(headers, bearer);
  } else {
    snprintf(key_header, key_len, "Api-Key: %s", db->api_key);
    bearer[0] = '\0';
    headers = curl_slist_append(headers, key_header);
    headers = curl_slist_append(
        headers, "X-Pinecone-API-Version: " BRIX_PINECONE_API_VERSION);
  }
  if (body) headers = curl_slist_append(headers, "Content-Type: application/json");
  headers = curl_slist_append(headers, "Accept: application/json");

  VdbBuf buf = {NULL, 0, 0, 0};
  CURL *c = db->curl;
  curl_easy_reset(c); // drops per-request options, keeps live connections
  curl_easy_setopt(c, CURLOPT_URL, url);
  curl_easy_setopt(c, CURLOPT_PROTOCOLS_STR, "http,https");
  curl_easy_setopt(c, CURLOPT_HTTPHEADER, headers);
  if (body) {
    curl_easy_setopt(c, CURLOPT_POSTFIELDS, body);
    curl_easy_setopt(c, CURLOPT_POSTFIELDSIZE, (long)strlen(body));
  } else {
    curl_easy_setopt(c, CURLOPT_HTTPGET, 1L);
  }
  if (method) curl_easy_setopt(c, CURLOPT_CUSTOMREQUEST, method);
  curl_easy_setopt(c, CURLOPT_WRITEFUNCTION, vdb_write_cb);
  curl_easy_setopt(c, CURLOPT_WRITEDATA, &buf);
  curl_easy_setopt(c, CURLOPT_NOSIGNAL, 1L);
  // Plain http is only allowed to loopback: never hand it (and the Api-Key)
  // to an http_proxy from the environment. https may still use HTTPS_PROXY,
  // which tunnels with CONNECT.
  if (strncmp(db->base_url, "http://", 7) == 0) {
    curl_easy_setopt(c, CURLOPT_NOPROXY, "*");
  }
  curl_easy_setopt(c, CURLOPT_CONNECTTIMEOUT_MS,
                   db->timeout_ms < 10000 ? db->timeout_ms : 10000L);
  curl_easy_setopt(c, CURLOPT_TIMEOUT_MS, db->timeout_ms);

  // Retry rate limits and transient server errors (every data-plane call of
  // both providers is idempotent). Connection failures and timeouts are not retried:
  // a down host fails fast.
  CURLcode rc;
  long status = 0;
  long attempt = 0;
  for (;;) {
    rc = curl_easy_perform(c);
    status = 0;
    curl_easy_getinfo(c, CURLINFO_RESPONSE_CODE, &status);
    int retryable = rc == CURLE_OK && (status == 429 || status == 502 ||
                                       status == 503 || status == 504);
    if (!retryable || attempt >= db->retries) break;
    // libcurl reports 0 both for "no Retry-After" and "Retry-After: 0", so
    // only a positive value overrides the backoff.
    curl_off_t retry_after = 0;
    long delay_ms;
    if (curl_easy_getinfo(c, CURLINFO_RETRY_AFTER, &retry_after) != CURLE_OK) {
      retry_after = 0;
    }
    if (retry_after > 0) {
      delay_ms = retry_after > BRIX_VDB_MAX_DELAY_MS / 1000
                     ? BRIX_VDB_MAX_DELAY_MS
                     : (long)retry_after * 1000;
    } else {
      // Exponential backoff with jitter: base * 2^attempt * [0.5, 1].
      long cap = BRIX_VDB_BACKOFF_BASE_MS << (attempt < 16 ? attempt : 16);
      if (cap > BRIX_VDB_MAX_DELAY_MS) cap = BRIX_VDB_MAX_DELAY_MS;
      delay_ms = cap / 2 + rand() % (cap / 2 + 1);
    }
    free(buf.data);
    buf = (VdbBuf){NULL, 0, 0, 0};
    struct timespec ts = {delay_ms / 1000, (delay_ms % 1000) * 1000000L};
    // A caught signal cuts the sleep short; finish the remaining delay.
    while (nanosleep(&ts, &ts) == -1 && errno == EINTR) {
    }
    attempt++;
  }
  db->last_status = rc == CURLE_OK ? status : 0;

  curl_slist_free_all(headers);
  memset(key_header, 0, key_len);
  memset(bearer, 0, key_len);
  free(key_header);
  free(bearer);
  free(url);

  JsonValue *result = NULL;
  if (buf.too_large) {
    vdb_set_error(db, "%s: response larger than %ld bytes", path,
                  BRIX_VDB_MAX_RESPONSE);
  } else if (rc != CURLE_OK) {
    vdb_set_error(db, "%s: %s", path, curl_easy_strerror(rc));
  } else if (status < 200 || status >= 300) {
    if (attempt > 0) {
      vdb_set_error(db, "%s: HTTP %ld after %ld %s: %.200s", path, status,
                    attempt, attempt == 1 ? "retry" : "retries",
                    buf.data ? buf.data : "");
    } else {
      vdb_set_error(db, "%s: HTTP %ld: %.200s", path, status,
                    buf.data ? buf.data : "");
    }
  } else {
    const char *p = buf.data ? buf.data : "";
    result = json_parse_value(&p, 0);
    if (result) {
      json_parse_skip_whitespace(&p);
      if (*p != '\0') {
        json_release(result);
        result = NULL;
      }
    }
    if (!result) vdb_set_error(db, "%s: response is not valid JSON", path);
  }
  // Milvus answers most errors with HTTP 200 and a non-zero "code".
  if (result && db->provider == BRIX_VDB_MILVUS) {
    JsonValue *code = vdb_field(result, "code");
    if (!code || code->tag != JSON_INT || code->data.as_int != 0) {
      JsonValue *msg = vdb_field(result, "message");
      if (!code || code->tag != JSON_INT) {
        vdb_set_error(db, "%s: response has no \"code\"", path);
      } else {
        vdb_set_error(db, "%s: code %ld: %.200s", path, code->data.as_int,
                      msg && msg->tag == JSON_STRING ? msg->data.as_string->data
                                                     : "");
      }
      json_release(msg);
      json_release(result);
      result = NULL;
    }
    json_release(code);
  }
  free(buf.data);
  return result;
}

static JsonValue *vdb_request(BrixVectorDB *db, const char *path,
                              const char *query, const char *body) {
  return vdb_request_at(db, NULL, "", path, query, body);
}

// Chroma implementations (end of this section). The public functions below
// run the checks both providers share, then dispatch on db->provider.
static JsonValue *chroma_stats(BrixVectorDB *db);
static JsonValue *chroma_query(BrixVectorDB *db, BrixEmbedding *q, long top_k,
                               JsonValue *filter);
static JsonValue *chroma_fetch(BrixVectorDB *db, BrixString *id);
static JsonValue *chroma_list(BrixVectorDB *db, long limit, BrixString *token);
static int chroma_write(BrixVectorDB *db, const char *path, long n,
                        BrixString **ids, BrixEmbedding **vecs,
                        JsonValue **metas, int single);
static long vdb_unsupported(BrixVectorDB *db, const char *method);
static JsonValue *wv_stats(BrixVectorDB *db);
static JsonValue *wv_query(BrixVectorDB *db, BrixEmbedding *q, long top_k,
                           JsonValue *filter);
static JsonValue *wv_fetch(BrixVectorDB *db, BrixString *id);
static JsonValue *wv_list(BrixVectorDB *db, long limit, BrixString *token);
static int wv_write(BrixVectorDB *db, int del, long n, BrixString **ids,
                    BrixEmbedding **vecs, JsonValue **metas, int single);
static JsonValue *mv_stats(BrixVectorDB *db);
static JsonValue *mv_query(BrixVectorDB *db, BrixEmbedding *q, long top_k,
                           JsonValue *filter);
static JsonValue *mv_fetch(BrixVectorDB *db, BrixString *id);
static JsonValue *mv_list(BrixVectorDB *db, long limit, BrixString *token);
static int mv_write(BrixVectorDB *db, int del, long n, BrixString **ids,
                    BrixEmbedding **vecs, JsonValue **metas, int single);

#define VDB_UPSERT_PATH(db)                                                  \
  ((db)->provider == BRIX_VDB_CHROMA     ? "/upsert"                         \
   : (db)->provider == BRIX_VDB_WEAVIATE ? "/v1/batch/objects"               \
   : (db)->provider == BRIX_VDB_MILVUS   ? "/v2/vectordb/entities/upsert"    \
                                         : "/vectors/upsert")
#define VDB_DELETE_PATH(db)                                                  \
  ((db)->provider == BRIX_VDB_CHROMA     ? "/delete"                         \
   : (db)->provider == BRIX_VDB_WEAVIATE ? "/v1/batch/objects"               \
   : (db)->provider == BRIX_VDB_MILVUS   ? "/v2/vectordb/entities/delete"    \
                                         : "/vectors/delete")

// db.stats() -> json? (Pinecone describe_index_stats: dimension,
// totalVectorCount, indexFullness, namespaces).
JsonValue *brix_vectordb_stats(BrixVectorDB *db) {
  vdb_begin(db);
  if (db->provider == BRIX_VDB_CHROMA) return chroma_stats(db);
  if (db->provider == BRIX_VDB_WEAVIATE) return wv_stats(db);
  if (db->provider == BRIX_VDB_MILVUS) return mv_stats(db);
  return vdb_request(db, "/describe_index_stats", NULL, "{}");
}

// Set `key` on `obj`, taking ownership of `val` (NULL is ignored).
static void vdb_put(JsonValue *obj, const char *key, JsonValue *val) {
  BrixString *k = str_new((char *)key);
  json_set(obj, k, val);
  string_release(k);
  json_release(val);
}

// Fetch `key` of `obj` as an owned reference, or NULL.
static JsonValue *vdb_field(JsonValue *obj, const char *key) {
  BrixString *k = str_new((char *)key);
  JsonValue *v = json_get(obj, k);
  string_release(k);
  return v;
}

// vdb_put for a caller's value (borrowed). 0 when json_set refused it
// (nested deeper than 64 levels): never send a request without the filter or
// metadata the caller asked for.
static int vdb_put_checked(JsonValue *obj, const char *key, JsonValue *val) {
  vdb_put(obj, key, json_retain(val));
  JsonValue *sent = vdb_field(obj, key);
  json_release(sent);
  return sent == val;
}

// Append "&name=<url-encoded value>" (no '&' when *q is empty) to *q.
// Returns 0 on out of memory.
static int vdb_add_param(BrixVectorDB *db, char **q, const char *name,
                         const char *value) {
  char *enc = curl_easy_escape(db->curl, value, 0);
  if (!enc) return 0;
  size_t old = *q ? strlen(*q) : 0;
  size_t n = old + strlen(name) + strlen(enc) + 3;
  char *p = realloc(*q, n);
  if (!p) {
    curl_free(enc);
    return 0;
  }
  snprintf(p + old, n - old, "%s%s=%s", old ? "&" : "", name, enc);
  curl_free(enc);
  *q = p;
  return 1;
}

// db.query(q, top_k[, filter]) -> json? : Pinecone POST /query. Returns the
// "matches" array ([{id, score, metadata}], best first, metadata included,
// values not). `filter` (a JSON object, NULL = none) is sent as-is.
JsonValue *brix_vectordb_query(BrixVectorDB *db, BrixEmbedding *q, long top_k,
                               JsonValue *filter) {
  vdb_begin(db);
  if (top_k < 1) {
    vdb_set_error(db, "/query: top_k must be >= 1, got %ld", top_k);
    return NULL;
  }
  for (long i = 0; i < q->dim; i++) {
    if (!isfinite(q->data[i])) {
      vdb_set_error(db, "/query: query vector contains NaN or Inf");
      return NULL;
    }
  }
  if (db->provider == BRIX_VDB_CHROMA) return chroma_query(db, q, top_k, filter);
  if (db->provider == BRIX_VDB_WEAVIATE) return wv_query(db, q, top_k, filter);
  if (db->provider == BRIX_VDB_MILVUS) return mv_query(db, q, top_k, filter);
  JsonValue *vec = json_array();
  for (long i = 0; i < q->dim; i++) {
    JsonValue *x = json_float(q->data[i]);
    json_array_push(vec, x);
    json_release(x);
  }
  JsonValue *req = json_object();
  vdb_put(req, "vector", vec);
  vdb_put(req, "topK", json_int(top_k));
  vdb_put(req, "includeMetadata", json_bool(1));
  vdb_put(req, "includeValues", json_bool(0));
  BrixString *ns = str_new(db->ns);
  vdb_put(req, "namespace", json_string(ns));
  string_release(ns);
  if (filter) {
    if (!vdb_put_checked(req, "filter", filter)) {
      json_release(req);
      vdb_set_error(db, "/query: filter is nested too deeply");
      return NULL;
    }
  }
  BrixString *body = json_stringify(req);
  json_release(req);

  JsonValue *resp = vdb_request(db, "/query", NULL, body->data);
  string_release(body);
  if (!resp) return NULL;
  JsonValue *matches = vdb_field(resp, "matches");
  json_release(resp);
  if (!matches || matches->tag != JSON_ARRAY) {
    json_release(matches);
    vdb_set_error(db, "/query: response has no \"matches\" array");
    return NULL;
  }
  return matches;
}

// db.fetch(id) -> json? : Pinecone GET /vectors/fetch. Returns the record
// {id, values, metadata}; nil with last_error "" when the id does not exist,
// nil with last_error set on a failure.
JsonValue *brix_vectordb_fetch(BrixVectorDB *db, BrixString *id) {
  vdb_begin(db);
  if (db->provider == BRIX_VDB_CHROMA) return chroma_fetch(db, id);
  if (db->provider == BRIX_VDB_WEAVIATE) return wv_fetch(db, id);
  if (db->provider == BRIX_VDB_MILVUS) return mv_fetch(db, id);
  char *query = NULL;
  if (!vdb_add_param(db, &query, "ids", id->data) ||
      (*db->ns && !vdb_add_param(db, &query, "namespace", db->ns))) {
    free(query);
    vdb_set_error(db, "/vectors/fetch: out of memory");
    return NULL;
  }
  JsonValue *resp = vdb_request(db, "/vectors/fetch", query, NULL);
  free(query);
  if (!resp) return NULL;
  JsonValue *vectors = vdb_field(resp, "vectors");
  json_release(resp);
  if (!vectors || vectors->tag != JSON_OBJECT) {
    json_release(vectors);
    vdb_set_error(db, "/vectors/fetch: response has no \"vectors\" object");
    return NULL;
  }
  JsonValue *rec = json_get(vectors, id);
  json_release(vectors);
  return rec;
}

// db.list([limit[, token]]) -> json? : Pinecone GET /vectors/list
// (serverless indexes). Returns the response as-is:
// {vectors: [{id}], pagination: {next}} (pagination is absent on the last
// page). limit LONG_MIN means "not given" (server default; codegen passes it
// when the argument is omitted); token "" or NULL = first page.
JsonValue *brix_vectordb_list(BrixVectorDB *db, long limit, BrixString *token) {
  vdb_begin(db);
  if (db->provider == BRIX_VDB_CHROMA) return chroma_list(db, limit, token);
  if (db->provider == BRIX_VDB_WEAVIATE) return wv_list(db, limit, token);
  if (db->provider == BRIX_VDB_MILVUS) return mv_list(db, limit, token);
  if (limit != LONG_MIN && limit < 1) {
    vdb_set_error(db, "/vectors/list: limit must be >= 1, got %ld", limit);
    return NULL;
  }
  char *query = NULL;
  int ok = 1;
  if (limit != LONG_MIN) {
    char num[32];
    snprintf(num, sizeof(num), "%ld", limit);
    ok = vdb_add_param(db, &query, "limit", num);
  }
  if (ok && token && token->len > 0) {
    ok = vdb_add_param(db, &query, "paginationToken", token->data);
  }
  if (ok && *db->ns) ok = vdb_add_param(db, &query, "namespace", db->ns);
  if (!ok) {
    free(query);
    vdb_set_error(db, "/vectors/list: out of memory");
    return NULL;
  }
  JsonValue *resp = vdb_request(db, "/vectors/list", query, NULL);
  free(query);
  return resp;
}

// ---- Fase 3: writes. Every write returns 1 on success, 0 on failure (the
// reason in last_error). Batches are validated up front, then serialized and
// sent one request at a time, within Pinecone's request limits; a failed
// request stops the batch and last_error says how many records were already
// written (earlier requests are not rolled back; a timed-out request may
// still have been applied, so the count is a lower bound). Upsert and delete
// are idempotent, so retrying the whole batch is safe.

#define BRIX_VDB_MAX_BATCH 1000
#define BRIX_VDB_MAX_BODY (2L * 1000 * 1000) // Pinecone: 2 MB per request

// Error about record `idx` of a batch (idx < 0: a single-record call).
static void vdb_record_error(BrixVectorDB *db, const char *path, long idx,
                             const char *msg) {
  if (idx < 0) {
    vdb_set_error(db, "%s: %s", path, msg);
  } else {
    vdb_set_error(db, "%s: record %ld: %s", path, idx, msg);
  }
}

static JsonValue *vdb_embedding_json(BrixEmbedding *e) {
  JsonValue *vec = json_array();
  for (long i = 0; i < e->dim; i++) {
    JsonValue *x = json_float(e->data[i]);
    json_array_push(vec, x);
    json_release(x);
  }
  return vec;
}

// 1 when `id` is a non-empty string (Pinecone ids are 1-512 characters).
static int vdb_check_id(BrixVectorDB *db, const char *path, long idx,
                        BrixString *id) {
  if (!id || id->len == 0) {
    vdb_record_error(db, path, idx, "id must be a non-empty string");
    return 0;
  }
  return 1;
}

// 1 when record `idx` can be sent: a non-empty id, finite values (JSON
// cannot carry NaN or Inf) and metadata that is NULL, JSON null or an object.
static int vdb_check_record(BrixVectorDB *db, long idx, BrixString *id,
                            BrixEmbedding *e, JsonValue *metadata) {
  const char *path = VDB_UPSERT_PATH(db);
  if (!vdb_check_id(db, path, idx, id)) return 0;
  for (long i = 0; i < e->dim; i++) {
    if (!isfinite(e->data[i])) {
      vdb_record_error(db, path, idx, "vector contains NaN or Inf");
      return 0;
    }
  }
  if (metadata && metadata->tag != JSON_NULL && metadata->tag != JSON_OBJECT) {
    vdb_record_error(db, path, idx, "metadata must be a JSON object");
    return 0;
  }
  return 1;
}

// Produces item `i` of a chunked request as JSON text (owned), or NULL with
// last_error set.
typedef BrixString *(*VdbItemFn)(BrixVectorDB *db, void *ctx, long i);

// Grow *buf to hold `need` bytes. 0 on out of memory.
static int vdb_reserve(char **buf, size_t *cap, size_t need) {
  if (need <= *cap) return 1;
  size_t c = *cap ? *cap : 4096;
  while (c < need) c *= 2;
  char *p = realloc(*buf, c);
  if (!p) return 0;
  *buf = p;
  *cap = c;
  return 1;
}

// A batch failed after `done` of `n` records: say so in last_error.
static void vdb_note_written(BrixVectorDB *db, long done, long n) {
  if (!db->last_error) return;
  char *err = db->last_error;
  db->last_error = NULL;
  vdb_set_error(db, "%s (%ld of %ld written)", err, done, n);
  free(err);
}

// POST `prefix` + items 0..n-1 joined by ',' + `suffix` to `path`, split
// into requests of at most BRIX_VDB_MAX_BATCH items and BRIX_VDB_MAX_BODY
// bytes. Items are produced lazily, so only one request body is in memory.
static int vdb_post_chunked(BrixVectorDB *db, const char *path,
                            const char *prefix, const char *suffix, long n,
                            VdbItemFn item, void *ctx) {
  size_t plen = strlen(prefix), slen = strlen(suffix);
  char *body = NULL;
  size_t cap = 0;
  BrixString *pending = NULL; // produced but did not fit the last request
  long done = 0;
  int ok = 1;
  while (ok && done < n) {
    long count = 0;
    size_t len = plen;
    if (!vdb_reserve(&body, &cap, plen + slen + 1)) {
      vdb_set_error(db, "%s: out of memory", path);
      ok = 0;
      break;
    }
    memcpy(body, prefix, plen);
    while (done + count < n && count < BRIX_VDB_MAX_BATCH) {
      BrixString *it = pending ? pending : item(db, ctx, done + count);
      pending = NULL;
      if (!it) {
        ok = 0;
        break;
      }
      size_t need = len + (count ? 1 : 0) + it->len + slen + 1;
      if (need > (size_t)BRIX_VDB_MAX_BODY) {
        if (count > 0) {
          pending = it; // starts the next request
          break;
        }
        string_release(it);
        char msg[96];
        snprintf(msg, sizeof(msg), "larger than the %ld-byte request limit",
                 BRIX_VDB_MAX_BODY);
        vdb_record_error(db, path, n > 1 ? done : -1, msg);
        ok = 0;
        break;
      }
      if (!vdb_reserve(&body, &cap, need)) {
        string_release(it);
        vdb_set_error(db, "%s: out of memory", path);
        ok = 0;
        break;
      }
      if (count) body[len++] = ',';
      memcpy(body + len, it->data, it->len);
      len += it->len;
      string_release(it);
      count++;
    }
    if (!ok) break;
    memcpy(body + len, suffix, slen + 1);
    JsonValue *resp = vdb_request(db, path, NULL, body);
    if (!resp) {
      ok = 0;
      break;
    }
    json_release(resp);
    done += count;
  }
  string_release(pending);
  free(body);
  if (!ok && n > 1) vdb_note_written(db, done, n);
  return ok;
}

// `],"namespace":"<ns>"}` closes a request body after its item list.
static BrixString *vdb_ns_suffix(BrixVectorDB *db) {
  BrixString *ns = str_new(db->ns);
  JsonValue *v = json_string(ns);
  string_release(ns);
  if (!v) return NULL; // never send "namespace":null
  BrixString *quoted = json_stringify(v);
  json_release(v);
  size_t blen = quoted->len + 32;
  char *buf = malloc(blen);
  if (!buf) {
    string_release(quoted);
    return NULL;
  }
  snprintf(buf, blen, "],\"namespace\":%s}", quoted->data);
  string_release(quoted);
  BrixString *out = str_new(buf);
  free(buf);
  return out;
}

typedef struct {
  BrixString **ids;
  BrixEmbedding **vecs;
  JsonValue **metas; // NULL = no metadata
  int single;        // 1: errors do not name a record index
} VdbUpsertCtx;

// Upsert item: {"id", "values"[, "metadata"]}.
static BrixString *vdb_upsert_item(BrixVectorDB *db, void *ctx, long i) {
  VdbUpsertCtx *c = ctx;
  JsonValue *metadata = c->metas ? c->metas[i] : NULL;
  JsonValue *rec = json_object();
  vdb_put(rec, "id", json_string(c->ids[i]));
  vdb_put(rec, "values", vdb_embedding_json(c->vecs[i]));
  if (metadata && metadata->tag == JSON_OBJECT) {
    if (!vdb_put_checked(rec, "metadata", metadata)) {
      json_release(rec);
      vdb_record_error(db, "/vectors/upsert", c->single ? -1 : i,
                       "metadata is nested too deeply");
      return NULL;
    }
  }
  BrixString *text = json_stringify(rec);
  json_release(rec);
  return text;
}

// Delete item: the id as a JSON string.
static BrixString *vdb_delete_item(BrixVectorDB *db, void *ctx, long i) {
  (void)db;
  JsonValue *v = json_string(((BrixString **)ctx)[i]);
  BrixString *text = json_stringify(v);
  json_release(v);
  return text;
}

static int vdb_send(BrixVectorDB *db, const char *path, const char *prefix,
                    long n, VdbItemFn item, void *ctx) {
  if (n == 0) return 1;
  BrixString *suffix = vdb_ns_suffix(db);
  if (!suffix) {
    vdb_set_error(db, "%s: out of memory", path);
    return 0;
  }
  int ok = vdb_post_chunked(db, path, prefix, suffix->data, n, item, ctx);
  string_release(suffix);
  return ok;
}

// db.upsert(id, embedding[, metadata]) -> int : POST /vectors/upsert.
// `metadata` NULL or JSON null = none.
long brix_vectordb_upsert(BrixVectorDB *db, BrixString *id, BrixEmbedding *e,
                          JsonValue *metadata) {
  vdb_begin(db);
  if (!vdb_check_record(db, -1, id, e, metadata)) return 0;
  if (db->provider == BRIX_VDB_CHROMA) {
    return chroma_write(db, "/upsert", 1, &id, &e, metadata ? &metadata : NULL,
                        1);
  }
  if (db->provider == BRIX_VDB_WEAVIATE) {
    return wv_write(db, 0, 1, &id, &e, metadata ? &metadata : NULL, 1);
  }
  if (db->provider == BRIX_VDB_MILVUS) {
    return mv_write(db, 0, 1, &id, &e, metadata ? &metadata : NULL, 1);
  }
  VdbUpsertCtx c = {&id, &e, metadata ? &metadata : NULL, 1};
  return vdb_send(db, "/vectors/upsert", "{\"vectors\":[", 1, vdb_upsert_item,
                  &c);
}

// db.upsert_batch(ids, embeddings[, metadata]) -> int. `ids` is a
// Vector<string>; `metadata` (NULL or JSON null = none) a JSON array with
// one object or null per record. Every record is checked before the first
// request.
long brix_vectordb_upsert_batch(BrixVectorDB *db, BrixVector *ids,
                                BrixEmbeddingBatch *vecs, JsonValue *metadata) {
  vdb_begin(db);
  const char *path = VDB_UPSERT_PATH(db);
  if (ids->len != vecs->len) {
    vdb_set_error(db, "%s: %ld ids but %ld embeddings", path, ids->len,
                  vecs->len);
    return 0;
  }
  JsonValue **metas = NULL;
  if (metadata && metadata->tag != JSON_NULL) {
    if (metadata->tag != JSON_ARRAY ||
        (long)metadata->data.as_array->len != ids->len) {
      vdb_set_error(db, "%s: metadata must be a JSON array with one entry per "
                    "id", path);
      return 0;
    }
    metas = metadata->data.as_array->items;
  }
  for (long i = 0; i < ids->len; i++) {
    if (!vdb_check_record(db, i, ((BrixString **)ids->data)[i], vecs->items[i],
                          metas ? metas[i] : NULL)) {
      return 0;
    }
  }
  if (db->provider == BRIX_VDB_CHROMA) {
    return chroma_write(db, path, ids->len, (BrixString **)ids->data,
                        vecs->items, metas, 0);
  }
  if (db->provider == BRIX_VDB_WEAVIATE) {
    return wv_write(db, 0, ids->len, (BrixString **)ids->data, vecs->items,
                    metas, 0);
  }
  if (db->provider == BRIX_VDB_MILVUS) {
    return mv_write(db, 0, ids->len, (BrixString **)ids->data, vecs->items,
                    metas, 0);
  }
  VdbUpsertCtx c = {(BrixString **)ids->data, vecs->items, metas, 0};
  return vdb_send(db, "/vectors/upsert", "{\"vectors\":[", ids->len,
                  vdb_upsert_item, &c);
}

// db.delete(id) -> int : POST /vectors/delete. Deleting a missing id succeeds.
long brix_vectordb_delete(BrixVectorDB *db, BrixString *id) {
  vdb_begin(db);
  if (!vdb_check_id(db, VDB_DELETE_PATH(db), -1, id)) return 0;
  if (db->provider == BRIX_VDB_CHROMA) {
    return chroma_write(db, "/delete", 1, &id, NULL, NULL, 1);
  }
  if (db->provider == BRIX_VDB_WEAVIATE) {
    return wv_write(db, 1, 1, &id, NULL, NULL, 1);
  }
  if (db->provider == BRIX_VDB_MILVUS) {
    return mv_write(db, 1, 1, &id, NULL, NULL, 1);
  }
  return vdb_send(db, "/vectors/delete", "{\"ids\":[", 1, vdb_delete_item, &id);
}

// db.delete_batch(ids) -> int, `ids` a Vector<string>. Every id is checked
// before the first request.
long brix_vectordb_delete_batch(BrixVectorDB *db, BrixVector *ids) {
  vdb_begin(db);
  BrixString **id_data = (BrixString **)ids->data;
  for (long i = 0; i < ids->len; i++) {
    if (!vdb_check_id(db, VDB_DELETE_PATH(db), i, id_data[i])) return 0;
  }
  if (db->provider == BRIX_VDB_CHROMA) {
    return chroma_write(db, "/delete", ids->len, id_data, NULL, NULL, 0);
  }
  if (db->provider == BRIX_VDB_WEAVIATE) {
    return wv_write(db, 1, ids->len, id_data, NULL, NULL, 0);
  }
  if (db->provider == BRIX_VDB_MILVUS) {
    return mv_write(db, 1, ids->len, id_data, NULL, NULL, 0);
  }
  return vdb_send(db, "/vectors/delete", "{\"ids\":[", ids->len,
                  vdb_delete_item, id_data);
}

// ---- Fase 4: index management. Both return 1 on success, 0 on failure.

// POST /vectors/delete {"deleteAll": true, "namespace": ns}.
static int vdb_delete_all(BrixVectorDB *db, BrixString *ns) {
  JsonValue *req = json_object();
  vdb_put(req, "deleteAll", json_bool(1));
  vdb_put(req, "namespace", json_string(ns));
  BrixString *body = json_stringify(req);
  json_release(req);
  JsonValue *resp = vdb_request(db, "/vectors/delete", NULL, body->data);
  string_release(body);
  if (!resp) return 0;
  json_release(resp);
  return 1;
}

// db.delete_namespace(ns) -> int : delete every vector in `ns` ("" = the
// default namespace). Clearing an empty or missing namespace succeeds.
long brix_vectordb_delete_namespace(BrixVectorDB *db, BrixString *ns) {
  vdb_begin(db);
  if (db->provider != BRIX_VDB_PINECONE) {
    return vdb_unsupported(db, "delete_namespace");
  }
  return vdb_delete_all(db, ns);
}

// db.reset_index() -> int : delete every vector in every namespace. Pinecone
// has no single call for this: the namespaces come from
// describe_index_stats, then each one is cleared. A failure stops the reset
// and last_error says how many namespaces were already cleared.
long brix_vectordb_reset_index(BrixVectorDB *db) {
  if (db->provider != BRIX_VDB_PINECONE) {
    vdb_begin(db);
    return vdb_unsupported(db, "reset_index");
  }
  JsonValue *stats = brix_vectordb_stats(db); // clears last_error
  if (!stats) return 0;
  JsonValue *nss = vdb_field(stats, "namespaces");
  json_release(stats);
  if (!nss || nss->tag != JSON_OBJECT) {
    json_release(nss);
    vdb_set_error(db, "/describe_index_stats: response has no \"namespaces\" "
                  "object");
    return 0;
  }
  long total = 0;
  for (JsonObjectEntry *e = nss->data.as_object->head; e; e = e->next) total++;
  long done = 0;
  int ok = 1;
  for (JsonObjectEntry *e = nss->data.as_object->head; e; e = e->next) {
    if (!vdb_delete_all(db, e->key)) {
      ok = 0;
      break;
    }
    done++;
  }
  json_release(nss);
  if (!ok && db->last_error) {
    char *err = db->last_error;
    db->last_error = NULL;
    vdb_set_error(db, "%s (%ld of %ld namespaces cleared)", err, done, total);
    free(err);
  }
  return ok;
}

// ---- Fase 5: per-connection settings. Copies of a VectorDB share them.

// db.set_timeout(ms) -> int : time limit of each request *attempt* (its
// connect phase is capped at 10 s). 0 + last_error if ms < 1. It does not
// bound a whole operation: one request can take up to
// (retries + 1) * ms + retries * 30 s of waiting, and a batch or
// reset_index sends one request per chunk or namespace.
long brix_vectordb_set_timeout(BrixVectorDB *db, long ms) {
  vdb_begin(db);
  if (ms < 1) {
    vdb_set_error(db, "set_timeout: ms must be >= 1, got %ld", ms);
    return 0;
  }
  db->timeout_ms = ms;
  return 1;
}

// db.set_retries(n) -> int : extra attempts on HTTP 429/502/503/504
// (0 disables retrying). 0 + last_error unless 0 <= n <= 10.
long brix_vectordb_set_retries(BrixVectorDB *db, long n) {
  vdb_begin(db);
  if (n < 0 || n > BRIX_VDB_MAX_RETRIES) {
    vdb_set_error(db, "set_retries: n must be between 0 and %ld, got %ld",
                  BRIX_VDB_MAX_RETRIES, n);
    return 0;
  }
  db->retries = n;
  return 1;
}

// db.last_status() -> int : HTTP status of the last response received by
// the last operation (200, 401, 429...); 0 when it got none (a local error,
// a connection failure or a timeout). An operation that sends several
// requests reports the last one: a batch whose second chunk fails locally
// returns 0 with last_status() still 200 from the first chunk. Settings
// calls are operations too and reset it to 0.
long brix_vectordb_last_status(BrixVectorDB *db) { return db->last_status; }

// ---- Grupo B Fase 1: Chroma (HTTP API v2: a `chroma run` server or Chroma
// Cloud). One connection is one collection, which must already exist: it is
// never created implicitly (Chroma would pick a distance space and index
// config the user did not choose). Its name is resolved to its id on the
// first operation and the id is cached; a 404 drops the cached id (the
// collection may have been recreated), and stats() always resolves again.
// Filters are Chroma "where" objects, sent as-is. Results keep the Pinecone
// shapes, except that a query match has "distance" (lower is closer, in the
// collection's space) instead of "score". No namespaces: delete_namespace and
// reset_index are not supported.

// ponytail: Chroma Cloud's per-write record quota; a local server allows more
// (GET /api/v2/pre-flight-checks), read it if 300 per request gets slow.
#define BRIX_CHROMA_MAX_BATCH 300

static long vdb_unsupported(BrixVectorDB *db, const char *method) {
  static const char *const names[] = {"pinecone", "chroma", "weaviate",
                                      "milvus"};
  vdb_set_error(db, "%s: not supported by %s", method, names[db->provider]);
  return 0;
}

static char *vdb_concat(const char *a, const char *b, const char *c) {
  size_t n = strlen(a) + strlen(b) + strlen(c) + 1;
  char *s = malloc(n);
  if (s) snprintf(s, n, "%s%s%s", a, b, c);
  return s;
}

static JsonValue *vdb_json_text(const char *text) {
  const char *p = text;
  return json_parse_value(&p, 0);
}

// Push `val` (owned) onto `arr`. 0 when json_array_push refused it (nested
// deeper than 64 levels).
static int vdb_push(JsonValue *arr, JsonValue *val) {
  size_t before = arr->data.as_array->len;
  json_array_push(arr, val);
  json_release(val);
  return arr->data.as_array->len > before;
}

// GET the collection by name and cache its id. Returns the collection object
// (owned), or NULL with last_error set.
static JsonValue *chroma_collection(BrixVectorDB *db) {
  char *enc = curl_easy_escape(db->curl, db->ns, 0);
  char *path = enc ? vdb_concat("/collections/", enc, "") : NULL;
  curl_free(enc);
  if (!path) {
    vdb_set_error(db, "out of memory");
    return NULL;
  }
  JsonValue *coll = vdb_request_at(db, NULL, db->db_path, path, NULL, NULL);
  if (!coll) {
    free(path);
    return NULL;
  }
  JsonValue *id = vdb_field(coll, "id");
  char *id_enc = NULL;
  if (id && id->tag == JSON_STRING && id->data.as_string->len > 0) {
    id_enc = curl_easy_escape(db->curl, id->data.as_string->data, 0);
  }
  json_release(id);
  char *coll_path =
      id_enc ? vdb_concat(db->db_path, "/collections/", id_enc) : NULL;
  curl_free(id_enc);
  if (!coll_path) {
    vdb_set_error(db, "%s: response has no collection \"id\"", path);
    free(path);
    json_release(coll);
    return NULL;
  }
  free(path);
  free(db->coll_path);
  db->coll_path = coll_path;
  return coll;
}

// POST `req` (owned) to the collection's `path` ("/query", "/get", ...).
static JsonValue *chroma_post(BrixVectorDB *db, const char *path,
                              JsonValue *req) {
  if (!db->coll_path) {
    JsonValue *coll = chroma_collection(db);
    if (!coll) {
      json_release(req);
      return NULL;
    }
    json_release(coll);
  }
  BrixString *body = json_stringify(req);
  json_release(req);
  JsonValue *resp = vdb_request_at(db, NULL, db->coll_path, path, NULL, body->data);
  string_release(body);
  if (!resp && db->last_status == 404) {
    // The collection may have been recreated under a new id: resolve the
    // name again on the next operation.
    free(db->coll_path);
    db->coll_path = NULL;
  }
  return resp;
}

// stats() -> {"dimension": int or null, "totalVectorCount": int}.
static JsonValue *chroma_stats(BrixVectorDB *db) {
  JsonValue *coll = chroma_collection(db);
  if (!coll) return NULL;
  JsonValue *count = vdb_request_at(db, NULL, db->coll_path, "/count", NULL, NULL);
  if (!count || count->tag != JSON_INT) {
    if (count) vdb_set_error(db, "/count: response is not an integer");
    json_release(count);
    json_release(coll);
    return NULL;
  }
  JsonValue *dim = vdb_field(coll, "dimension");
  json_release(coll);
  JsonValue *out = json_object();
  vdb_put(out, "dimension", dim ? dim : json_null());
  vdb_put(out, "totalVectorCount", count);
  return out;
}

// Owned array `resp[key]` (an array), or NULL. `row` >= 0 takes that row of
// a nested array (query results have one row per query embedding).
static JsonValue *chroma_column(JsonValue *resp, const char *key, long row) {
  JsonValue *col = vdb_field(resp, key);
  if (col && row >= 0) {
    JsonValue *r = json_index(col, row);
    json_release(col);
    col = r;
  }
  if (col && col->tag != JSON_ARRAY) {
    json_release(col);
    col = NULL;
  }
  return col;
}

// Record i of the parallel columns: {id[, key: vals[i]][, metadata}}.
// Metadata is left out when null, as Pinecone does.
static JsonValue *chroma_record(JsonValue *ids, long i, const char *key,
                                JsonValue *vals, JsonValue *metas) {
  JsonValue *rec = json_object();
  vdb_put(rec, "id", json_index(ids, i));
  if (vals) vdb_put(rec, key, json_index(vals, i));
  JsonValue *m = metas ? json_index(metas, i) : NULL;
  if (m && m->tag == JSON_OBJECT) {
    vdb_put(rec, "metadata", m);
  } else {
    json_release(m);
  }
  return rec;
}

static JsonValue *chroma_query(BrixVectorDB *db, BrixEmbedding *q, long top_k,
                               JsonValue *filter) {
  JsonValue *qs = json_array();
  vdb_push(qs, vdb_embedding_json(q));
  JsonValue *req = json_object();
  vdb_put(req, "query_embeddings", qs);
  vdb_put(req, "n_results", json_int(top_k));
  vdb_put(req, "include", vdb_json_text("[\"metadatas\",\"distances\"]"));
  if (filter && !vdb_put_checked(req, "where", filter)) {
    json_release(req);
    vdb_set_error(db, "/query: filter is nested too deeply");
    return NULL;
  }
  JsonValue *resp = chroma_post(db, "/query", req);
  if (!resp) return NULL;
  JsonValue *ids = chroma_column(resp, "ids", 0);
  JsonValue *dists = chroma_column(resp, "distances", 0);
  JsonValue *metas = chroma_column(resp, "metadatas", 0);
  json_release(resp);
  JsonValue *matches = NULL;
  if (ids) {
    matches = json_array();
    for (long i = 0; i < json_array_len(ids); i++) {
      vdb_push(matches, chroma_record(ids, i, "distance", dists, metas));
    }
  } else {
    vdb_set_error(db, "/query: response has no \"ids\" array");
  }
  json_release(ids);
  json_release(dists);
  json_release(metas);
  return matches;
}

// fetch(id) -> {id, values, metadata}; nil with last_error "" when missing.
static JsonValue *chroma_fetch(BrixVectorDB *db, BrixString *id) {
  JsonValue *want = json_array();
  vdb_push(want, json_string(id));
  JsonValue *req = json_object();
  vdb_put(req, "ids", want);
  vdb_put(req, "include", vdb_json_text("[\"embeddings\",\"metadatas\"]"));
  JsonValue *resp = chroma_post(db, "/get", req);
  if (!resp) return NULL;
  JsonValue *ids = chroma_column(resp, "ids", -1);
  JsonValue *vals = chroma_column(resp, "embeddings", -1);
  JsonValue *metas = chroma_column(resp, "metadatas", -1);
  json_release(resp);
  JsonValue *rec = NULL;
  if (!ids) {
    vdb_set_error(db, "/get: response has no \"ids\" array");
  } else if (json_array_len(ids) > 0) {
    rec = chroma_record(ids, 0, "values", vals, metas);
  }
  json_release(ids);
  json_release(vals);
  json_release(metas);
  return rec;
}

// list([limit[, token]]) -> {vectors: [{id}], pagination: {next}}, the
// Pinecone page shape. Pages come from /get limit+offset; the token is the
// next offset, and "pagination" is left out once a page comes back short.
static JsonValue *chroma_list(BrixVectorDB *db, long limit, BrixString *token) {
  if (limit == LONG_MIN) limit = 100; // Pinecone's default page size
  if (limit < 1) {
    vdb_set_error(db, "/get: limit must be >= 1, got %ld", limit);
    return NULL;
  }
  long offset = 0;
  if (token && token->len > 0) {
    char *end;
    errno = 0;
    offset = strtol(token->data, &end, 10);
    if (*end || errno || offset < 0 || !isdigit((unsigned char)token->data[0])) {
      vdb_set_error(db, "/get: invalid pagination token");
      return NULL;
    }
  }
  JsonValue *req = json_object();
  vdb_put(req, "limit", json_int(limit));
  vdb_put(req, "offset", json_int(offset));
  vdb_put(req, "include", json_array());
  JsonValue *resp = chroma_post(db, "/get", req);
  if (!resp) return NULL;
  JsonValue *ids = chroma_column(resp, "ids", -1);
  json_release(resp);
  if (!ids) {
    vdb_set_error(db, "/get: response has no \"ids\" array");
    return NULL;
  }
  long n = json_array_len(ids);
  JsonValue *vectors = json_array();
  for (long i = 0; i < n; i++) {
    vdb_push(vectors, chroma_record(ids, i, NULL, NULL, NULL));
  }
  json_release(ids);
  JsonValue *page = json_object();
  vdb_put(page, "vectors", vectors);
  if (n == limit) {
    char next[32];
    snprintf(next, sizeof(next), "%ld", offset + n);
    BrixString *s = str_new(next);
    JsonValue *pag = json_object();
    vdb_put(pag, "next", json_string(s));
    string_release(s);
    vdb_put(page, "pagination", pag);
  }
  return page;
}

// POST ids[0..n) to `path` ("/upsert" with `vecs` and optional `metas`,
// "/delete" with only ids) in requests of at most BRIX_CHROMA_MAX_BATCH
// records. Records were checked by the caller; a failed request stops the
// batch with "(N of M written)", like Pinecone. Unlike Pinecone, a Chroma
// upsert merges the metadata into the record's existing metadata (null or no
// metadata keeps it, a key set to null removes that key).
static int chroma_write(BrixVectorDB *db, const char *path, long n,
                        BrixString **ids, BrixEmbedding **vecs,
                        JsonValue **metas, int single) {
  long done = 0;
  int ok = 1;
  while (ok && done < n) {
    long count = n - done < BRIX_CHROMA_MAX_BATCH ? n - done
                                                   : BRIX_CHROMA_MAX_BATCH;
    JsonValue *req = json_object();
    JsonValue *jids = json_array();
    JsonValue *jvecs = vecs ? json_array() : NULL;
    JsonValue *jmetas = metas ? json_array() : NULL;
    for (long i = done; ok && i < done + count; i++) {
      vdb_push(jids, json_string(ids[i]));
      if (jvecs) vdb_push(jvecs, vdb_embedding_json(vecs[i]));
      if (jmetas) {
        JsonValue *m = metas[i];
        int none = !m || m->tag != JSON_OBJECT;
        if (!vdb_push(jmetas, none ? json_null() : json_retain(m))) {
          vdb_record_error(db, path, single ? -1 : i,
                           "metadata is nested too deeply");
          ok = 0;
        }
      }
    }
    vdb_put(req, "ids", jids);
    if (jvecs) vdb_put(req, "embeddings", jvecs);
    // One level deeper than the per-record check: json_set can still refuse
    // the whole array, and the request must never go out without it.
    if (jmetas && ok && !vdb_put_checked(req, "metadatas", jmetas)) {
      vdb_record_error(db, path, -1, "metadata is nested too deeply");
      ok = 0;
    }
    json_release(jmetas);
    if (!ok) {
      json_release(req);
      break;
    }
    JsonValue *resp = chroma_post(db, path, req);
    if (!resp) {
      ok = 0;
      break;
    }
    json_release(resp);
    done += count;
  }
  if (!ok && n > 1) vdb_note_written(db, done, n);
  return ok;
}

// ---- Grupo B Fase 2: Weaviate (REST /v1 + GraphQL). One connection is one
// existing class with a single (unnamed) vector: it is never created
// implicitly (a write to a missing class would create one through
// auto-schema), so upserts, query and stats read the schema first; fetch,
// list and delete use the class spelling cached on first use (a 404 drops
// it). Weaviate ids must be UUIDs, so a
// Brix id becomes UUIDv5(NAMESPACE_DNS, id) -- what the Weaviate clients'
// generate_uuid5(id) returns -- and is also stored in the "brix_id" property,
// which reads turn back into the id (objects without it report their UUID).
// Metadata is the object's other properties. Results keep the Pinecone
// shapes; a query match has "distance" like Chroma. Filters are Weaviate
// "where" objects (path / operator / value*), sent as GraphQL input. No
// namespaces: delete_namespace and reset_index are not supported.

// ponytail: the batch size Weaviate recommends; no request-size cap.
#define BRIX_WEAVIATE_MAX_BATCH 100

typedef struct {
  char *data;
  size_t len, cap;
  int oom;
} VdbText;

static void vt_add(VdbText *t, const char *s) {
  size_t n = strlen(s);
  if (t->oom) return;
  if (!vdb_reserve(&t->data, &t->cap, t->len + n + 1)) {
    t->oom = 1;
    return;
  }
  memcpy(t->data + t->len, s, n + 1);
  t->len += n;
}

static void vt_json(VdbText *t, JsonValue *v) {
  BrixString *s = json_stringify(v);
  if (!s) {
    t->oom = 1;
    return;
  }
  vt_add(t, s->data);
  string_release(s);
}

// `obj[key]` (owned) with `obj` released: lets lookups chain.
static JsonValue *vdb_take(JsonValue *obj, const char *key) {
  JsonValue *v = vdb_field(obj, key);
  json_release(obj);
  return v;
}

static JsonValue *vdb_take_index(JsonValue *arr, long i) {
  JsonValue *v = json_index(arr, i);
  json_release(arr);
  return v;
}

static uint32_t vdb_rol(uint32_t x, int n) { return (x << n) | (x >> (32 - n)); }

// SHA-1 (RFC 3174), only to derive UUIDv5 ids. 0 on out of memory.
static int vdb_sha1(const unsigned char *msg, size_t len, unsigned char out[20]) {
  uint32_t h[5] = {0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0};
  size_t total = ((len + 8) / 64 + 1) * 64;
  unsigned char *m = calloc(total, 1);
  if (!m) return 0;
  memcpy(m, msg, len);
  m[len] = 0x80;
  uint64_t bits = (uint64_t)len * 8;
  for (int i = 0; i < 8; i++) m[total - 1 - i] = (unsigned char)(bits >> (8 * i));
  for (size_t off = 0; off < total; off += 64) {
    uint32_t w[80];
    for (int i = 0; i < 16; i++) {
      const unsigned char *p = m + off + 4 * i;
      w[i] = (uint32_t)p[0] << 24 | (uint32_t)p[1] << 16 | (uint32_t)p[2] << 8 | p[3];
    }
    for (int i = 16; i < 80; i++) {
      w[i] = vdb_rol(w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16], 1);
    }
    uint32_t a = h[0], b = h[1], c = h[2], d = h[3], e = h[4];
    for (int i = 0; i < 80; i++) {
      uint32_t f, k;
      if (i < 20) {
        f = (b & c) | (~b & d);
        k = 0x5A827999;
      } else if (i < 40) {
        f = b ^ c ^ d;
        k = 0x6ED9EBA1;
      } else if (i < 60) {
        f = (b & c) | (b & d) | (c & d);
        k = 0x8F1BBCDC;
      } else {
        f = b ^ c ^ d;
        k = 0xCA62C1D6;
      }
      uint32_t t = vdb_rol(a, 5) + f + e + k + w[i];
      e = d;
      d = c;
      c = vdb_rol(b, 30);
      b = a;
      a = t;
    }
    h[0] += a;
    h[1] += b;
    h[2] += c;
    h[3] += d;
    h[4] += e;
  }
  free(m);
  for (int i = 0; i < 5; i++) {
    out[4 * i] = (unsigned char)(h[i] >> 24);
    out[4 * i + 1] = (unsigned char)(h[i] >> 16);
    out[4 * i + 2] = (unsigned char)(h[i] >> 8);
    out[4 * i + 3] = (unsigned char)h[i];
  }
  return 1;
}

// UUIDv5(NAMESPACE_DNS, id) as text. 0 on out of memory.
static int wv_uuid(BrixString *id, char out[37]) {
  static const unsigned char ns[16] = {0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad,
                                       0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0,
                                       0x4f, 0xd4, 0x30, 0xc8};
  size_t n = 16 + (size_t)id->len;
  unsigned char *m = malloc(n);
  if (!m) return 0;
  memcpy(m, ns, 16);
  memcpy(m + 16, id->data, id->len);
  unsigned char h[20];
  int ok = vdb_sha1(m, n, h);
  free(m);
  if (!ok) return 0;
  h[6] = (h[6] & 0x0f) | 0x50;
  h[8] = (h[8] & 0x3f) | 0x80;
  snprintf(out, 37,
           "%02x%02x%02x%02x-%02x%02x-%02x%02x-%02x%02x-%02x%02x%02x%02x%02x%02x",
           h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7], h[8], h[9], h[10],
           h[11], h[12], h[13], h[14], h[15]);
  return 1;
}

// GET /v1/schema/<class>. Caches the class spelling in coll_path. Returns
// the class schema (owned), or NULL with last_error set.
static JsonValue *wv_schema(BrixVectorDB *db) {
  char *path = vdb_concat("/v1/schema/", db->ns, "");
  if (!path) {
    vdb_set_error(db, "out of memory");
    return NULL;
  }
  JsonValue *schema = vdb_request_at(db, NULL, "", path, NULL, NULL);
  if (!schema && db->last_status == 404) {
    vdb_set_error(db, "%s: class does not exist", path);
  }
  JsonValue *cls = schema ? vdb_field(schema, "class") : NULL;
  if (schema && !(cls && cls->tag == JSON_STRING &&
                  vdb_is_name(cls->data.as_string->data))) {
    vdb_set_error(db, "%s: response has no \"class\" name", path);
    json_release(schema);
    schema = NULL;
  }
  if (schema) {
    char *name = strdup(cls->data.as_string->data);
    if (!name) {
      vdb_set_error(db, "out of memory");
      json_release(schema);
      schema = NULL;
    } else {
      free(db->coll_path);
      db->coll_path = name;
    }
  }
  json_release(cls);
  free(path);
  return schema;
}

static int wv_resolve(BrixVectorDB *db) {
  if (db->coll_path) return 1;
  JsonValue *schema = wv_schema(db);
  json_release(schema);
  return schema != NULL;
}

// A request on the resolved class: a 404 drops the cached class (it may have
// been deleted).
static JsonValue *wv_request(BrixVectorDB *db, const char *method,
                             const char *path, const char *query,
                             const char *body) {
  JsonValue *resp = vdb_request_at(db, method, "", path, query, body);
  if (!resp && db->last_status == 404) {
    free(db->coll_path);
    db->coll_path = NULL;
  }
  return resp;
}

// POST /v1/graphql. Returns "data" (owned), or NULL with last_error set; a
// GraphQL error comes back with HTTP 200 and an "errors" array.
static JsonValue *wv_graphql(BrixVectorDB *db, VdbText *query) {
  if (query->oom) {
    free(query->data);
    vdb_set_error(db, "/v1/graphql: out of memory");
    return NULL;
  }
  BrixString *q = str_new(query->data);
  free(query->data);
  JsonValue *req = json_object();
  vdb_put(req, "query", json_string(q));
  string_release(q);
  BrixString *body = json_stringify(req);
  json_release(req);
  JsonValue *resp = vdb_request_at(db, NULL, "", "/v1/graphql", NULL, body->data);
  string_release(body);
  if (!resp) return NULL;
  JsonValue *msg = vdb_take(vdb_take_index(vdb_field(resp, "errors"), 0), "message");
  if (msg) {
    vdb_set_error(db, "/v1/graphql: %s",
                  msg->tag == JSON_STRING ? msg->data.as_string->data : "error");
    json_release(msg);
    json_release(resp);
    return NULL;
  }
  return vdb_take(resp, "data");
}

// The properties of `props` but "brix_id", "_additional" and nulls; NULL when
// none remain.
static JsonValue *wv_meta(JsonValue *props) {
  if (!props || props->tag != JSON_OBJECT) return NULL;
  JsonValue *meta = NULL;
  for (JsonObjectEntry *e = props->data.as_object->head; e; e = e->next) {
    if (!e->val || e->val->tag == JSON_NULL ||
        strcmp(e->key->data, "brix_id") == 0 ||
        strcmp(e->key->data, "_additional") == 0) {
      continue;
    }
    if (!meta) meta = json_object();
    json_set(meta, e->key, e->val);
  }
  return meta;
}

// The Brix id of an object: its "brix_id" property, else its UUID.
static JsonValue *wv_id(JsonValue *props, JsonValue *uuid) {
  JsonValue *id = vdb_field(props, "brix_id");
  if (id && id->tag == JSON_STRING) return id;
  json_release(id);
  return json_retain(uuid);
}

// A REST object {id, properties, vector} as {id, values[, metadata]}.
static JsonValue *wv_record(JsonValue *obj) {
  JsonValue *props = vdb_field(obj, "properties");
  JsonValue *uuid = vdb_field(obj, "id");
  JsonValue *rec = json_object();
  vdb_put(rec, "id", wv_id(props, uuid));
  vdb_put(rec, "values", vdb_field(obj, "vector"));
  vdb_put(rec, "metadata", wv_meta(props));
  json_release(props);
  json_release(uuid);
  return rec;
}

// stats() -> {"dimension": null, "totalVectorCount": int} (Aggregate count;
// Weaviate does not report a class dimension).
static JsonValue *wv_stats(BrixVectorDB *db) {
  JsonValue *schema = wv_schema(db);
  if (!schema) return NULL;
  json_release(schema);
  VdbText q = {0};
  vt_add(&q, "{ Aggregate { ");
  vt_add(&q, db->coll_path);
  vt_add(&q, " { meta { count } } } }");
  JsonValue *data = wv_graphql(db, &q);
  if (!data) return NULL;
  JsonValue *agg = vdb_take(vdb_take(data, "Aggregate"), db->coll_path);
  JsonValue *count = vdb_take(vdb_take(vdb_take_index(agg, 0), "meta"), "count");
  if (!count || count->tag != JSON_INT) {
    json_release(count);
    vdb_set_error(db, "/v1/graphql: response has no count");
    return NULL;
  }
  JsonValue *out = json_object();
  vdb_put(out, "dimension", json_null());
  vdb_put(out, "totalVectorCount", count);
  return out;
}

// A JSON filter as GraphQL input: names as keys, an "operator" value as an
// enum, everything else as its JSON literal (GraphQL shares JSON's string
// escapes). 0 when it is not expressible.
static int wv_gql_value(VdbText *t, JsonValue *v, int is_enum, int depth) {
  if (!v || depth > 32) return 0;
  if (is_enum) {
    if (v->tag != JSON_STRING || !vdb_is_name(v->data.as_string->data)) return 0;
    vt_add(t, v->data.as_string->data);
    return 1;
  }
  if (v->tag == JSON_OBJECT) {
    vt_add(t, "{");
    for (JsonObjectEntry *e = v->data.as_object->head; e; e = e->next) {
      if (!vdb_is_name(e->key->data)) return 0;
      vt_add(t, e->key->data);
      vt_add(t, ": ");
      if (!wv_gql_value(t, e->val, strcmp(e->key->data, "operator") == 0,
                        depth + 1)) {
        return 0;
      }
      if (e->next) vt_add(t, ", ");
    }
    vt_add(t, "}");
    return 1;
  }
  if (v->tag == JSON_ARRAY) {
    vt_add(t, "[");
    JsonArray *a = v->data.as_array;
    for (size_t i = 0; i < a->len; i++) {
      if (i) vt_add(t, ", ");
      if (!wv_gql_value(t, a->items[i], 0, depth + 1)) return 0;
    }
    vt_add(t, "]");
    return 1;
  }
  vt_json(t, v);
  return 1;
}

static int wv_scalar_type(const char *t) {
  static const char *const scalars[] = {"text", "int", "number", "boolean",
                                        "date", "uuid", "blob"};
  size_t n = strlen(t);
  if (n > 2 && strcmp(t + n - 2, "[]") == 0) n -= 2; // arrays of a scalar
  for (size_t i = 0; i < sizeof(scalars) / sizeof(scalars[0]); i++) {
    if (strlen(scalars[i]) == n && strncmp(t, scalars[i], n) == 0) return 1;
  }
  return 0;
}

static JsonValue *wv_query(BrixVectorDB *db, BrixEmbedding *q, long top_k,
                           JsonValue *filter) {
  if (filter && filter->tag != JSON_OBJECT) {
    vdb_set_error(db, "/v1/graphql: filter must be a Weaviate where object");
    return NULL;
  }
  // The schema names the properties to select (GraphQL has no wildcard); it
  // is read on every query since auto-schema adds properties on writes.
  JsonValue *schema = wv_schema(db);
  if (!schema) return NULL;
  VdbText t = {0};
  char num[48];
  vt_add(&t, "{ Get { ");
  vt_add(&t, db->coll_path);
  vt_add(&t, "(nearVector: {vector: [");
  for (long i = 0; i < q->dim; i++) {
    snprintf(num, sizeof(num), "%s%.17g", i ? ", " : "", q->data[i]);
    vt_add(&t, num);
  }
  snprintf(num, sizeof(num), "]}, limit: %ld", top_k);
  vt_add(&t, num);
  if (filter) {
    vt_add(&t, ", where: ");
    if (!wv_gql_value(&t, filter, 0, 0)) {
      free(t.data);
      json_release(schema);
      vdb_set_error(db, "/v1/graphql: filter must be a Weaviate where object "
                    "(names as keys, \"operator\" a name)");
      return NULL;
    }
  }
  vt_add(&t, ") { ");
  // Scalar types only: object, geoCoordinates, phoneNumber and references
  // need a sub-selection.
  JsonValue *props = vdb_take(schema, "properties");
  for (long i = 0; i < json_array_len(props); i++) {
    JsonValue *p = json_index(props, i);
    JsonValue *name = vdb_field(p, "name");
    JsonValue *type = vdb_take_index(vdb_field(p, "dataType"), 0);
    if (name && name->tag == JSON_STRING && type && type->tag == JSON_STRING &&
        vdb_is_name(name->data.as_string->data) &&
        wv_scalar_type(type->data.as_string->data)) {
      vt_add(&t, name->data.as_string->data);
      vt_add(&t, " ");
    }
    json_release(type);
    json_release(name);
    json_release(p);
  }
  json_release(props);
  vt_add(&t, "_additional { id distance } } } }");

  JsonValue *hits = vdb_take(vdb_take(wv_graphql(db, &t), "Get"), db->coll_path);
  if (!hits || hits->tag != JSON_ARRAY) {
    json_release(hits);
    if (!db->last_error) vdb_set_error(db, "/v1/graphql: response has no hits");
    return NULL;
  }
  JsonValue *matches = json_array();
  for (long i = 0; i < json_array_len(hits); i++) {
    JsonValue *hit = json_index(hits, i);
    JsonValue *add = vdb_field(hit, "_additional");
    JsonValue *uuid = vdb_field(add, "id");
    JsonValue *m = json_object();
    vdb_put(m, "id", wv_id(hit, uuid));
    vdb_put(m, "distance", vdb_field(add, "distance"));
    vdb_put(m, "metadata", wv_meta(hit));
    vdb_push(matches, m);
    json_release(uuid);
    json_release(add);
    json_release(hit);
  }
  json_release(hits);
  return matches;
}

// "/v1/objects/<class>/<uuid of id>" (owned), or NULL on out of memory.
static char *wv_object_path(BrixVectorDB *db, BrixString *id) {
  char uuid[37];
  if (!wv_uuid(id, uuid)) return NULL;
  size_t n = strlen(db->coll_path) + 64;
  char *path = malloc(n);
  if (path) snprintf(path, n, "/v1/objects/%s/%s", db->coll_path, uuid);
  return path;
}

// fetch(id) -> {id, values, metadata}; nil with last_error "" when missing.
static JsonValue *wv_fetch(BrixVectorDB *db, BrixString *id) {
  if (!wv_resolve(db)) return NULL;
  char *path = wv_object_path(db, id);
  if (!path) {
    vdb_set_error(db, "out of memory");
    return NULL;
  }
  JsonValue *obj = wv_request(db, NULL, path, "include=vector", NULL);
  free(path);
  if (!obj) {
    if (db->last_status == 404) {
      free(db->last_error); // not found is not an error
      db->last_error = NULL;
    }
    return NULL;
  }
  JsonValue *rec = wv_record(obj);
  json_release(obj);
  return rec;
}

// list([limit[, token]]) -> {vectors: [{id}], pagination: {next}}: GET
// /v1/objects with the class, a limit and the `after` cursor (the last UUID
// of the previous page, which is the token).
static JsonValue *wv_list(BrixVectorDB *db, long limit, BrixString *token) {
  if (limit == LONG_MIN) limit = 100; // Pinecone's default page size
  if (limit < 1) {
    vdb_set_error(db, "/v1/objects: limit must be >= 1, got %ld", limit);
    return NULL;
  }
  if (!wv_resolve(db)) return NULL;
  char num[32];
  snprintf(num, sizeof(num), "%ld", limit);
  char *query = NULL;
  int ok = vdb_add_param(db, &query, "class", db->coll_path) &&
           vdb_add_param(db, &query, "limit", num);
  if (ok && token && token->len > 0) {
    ok = vdb_add_param(db, &query, "after", token->data);
  }
  if (!ok) {
    free(query);
    vdb_set_error(db, "/v1/objects: out of memory");
    return NULL;
  }
  JsonValue *resp = wv_request(db, NULL, "/v1/objects", query, NULL);
  free(query);
  JsonValue *objs = vdb_take(resp, "objects");
  if (!objs || objs->tag != JSON_ARRAY) {
    json_release(objs);
    if (!db->last_error) {
      vdb_set_error(db, "/v1/objects: response has no \"objects\" array");
    }
    return NULL;
  }
  long n = json_array_len(objs);
  JsonValue *vectors = json_array();
  JsonValue *last = NULL;
  for (long i = 0; i < n; i++) {
    JsonValue *obj = json_index(objs, i);
    JsonValue *props = vdb_field(obj, "properties");
    json_release(last);
    last = vdb_field(obj, "id");
    JsonValue *rec = json_object();
    vdb_put(rec, "id", wv_id(props, last));
    vdb_push(vectors, rec);
    json_release(props);
    json_release(obj);
  }
  json_release(objs);
  JsonValue *page = json_object();
  vdb_put(page, "vectors", vectors);
  if (n == limit && last && last->tag == JSON_STRING) {
    JsonValue *pag = json_object();
    vdb_put(pag, "next", json_retain(last));
    vdb_put(page, "pagination", pag);
  }
  json_release(last);
  return page;
}

// One batch object {class, id, properties: {brix_id, ...metadata}, vector}.
static JsonValue *wv_object(BrixVectorDB *db, BrixString *id, BrixEmbedding *e,
                            JsonValue *meta, const char *uuid) {
  JsonValue *props = json_object();
  vdb_put(props, "brix_id", json_string(id));
  size_t want = 1;
  if (meta && meta->tag == JSON_OBJECT) {
    for (JsonObjectEntry *x = meta->data.as_object->head; x; x = x->next) {
      json_set(props, x->key, x->val); // refuses values nested too deeply
      want++;
    }
  }
  JsonValue *obj = json_object();
  BrixString *cls = str_new(db->coll_path);
  BrixString *u = str_new((char *)uuid);
  vdb_put(obj, "class", json_string(cls));
  vdb_put(obj, "id", json_string(u));
  string_release(cls);
  string_release(u);
  vdb_put(obj, "vector", vdb_embedding_json(e));
  int ok = props->data.as_object->count == want &&
           vdb_put_checked(obj, "properties", props);
  json_release(props);
  if (!ok) {
    json_release(obj);
    return NULL;
  }
  return obj;
}

// The error of item `i` of a batch response, or NULL (borrowed text in *buf).
static const char *wv_item_error(JsonValue *resp, long i, char *buf, size_t n) {
  JsonValue *msg = vdb_take(
      vdb_take_index(
          vdb_take(vdb_take(vdb_take(json_index(resp, i), "result"), "errors"),
                   "error"),
          0),
      "message");
  if (!msg) return NULL;
  snprintf(buf, n, "%s",
           msg->tag == JSON_STRING ? msg->data.as_string->data : "failed");
  json_release(msg);
  return buf;
}

// Upsert (POST /v1/batch/objects, which replaces whole objects) or delete
// (DELETE /v1/batch/objects matching the ids) ids[0..n), in requests of
// BRIX_WEAVIATE_MAX_BATCH. Weaviate answers a batch with 200 and an error
// per failed object: the first one stops the batch, and "(N of M written)"
// counts the objects that did succeed.
static int wv_write(BrixVectorDB *db, int del, long n, BrixString **ids,
                    BrixEmbedding **vecs, JsonValue **metas, int single) {
  const char *path = "/v1/batch/objects";
  if (metas) {
    BrixString *key = str_new("brix_id");
    for (long i = 0; i < n; i++) {
      JsonValue *clash = json_get(metas[i], key);
      json_release(clash);
      if (clash) {
        string_release(key);
        vdb_record_error(db, path, single ? -1 : i,
                         "metadata must not contain \"brix_id\" (it holds the id)");
        return 0;
      }
    }
    string_release(key);
  }
  if (n == 0) return 1;
  long done = 0;
  // An upsert reads the schema fresh: the class may have been deleted since
  // it was cached, and auto-schema would silently recreate it.
  int ok;
  if (del) {
    ok = wv_resolve(db);
  } else {
    JsonValue *schema = wv_schema(db);
    ok = schema != NULL;
    json_release(schema);
  }
  while (ok && done < n) {
    long count = n - done < BRIX_WEAVIATE_MAX_BATCH ? n - done
                                                     : BRIX_WEAVIATE_MAX_BATCH;
    JsonValue *items = json_array();
    for (long i = done; ok && i < done + count; i++) {
      char uuid[37];
      if (!wv_uuid(ids[i], uuid)) {
        vdb_set_error(db, "%s: out of memory", path);
        ok = 0;
      } else if (del) {
        BrixString *u = str_new(uuid);
        vdb_push(items, json_string(u));
        string_release(u);
      } else {
        JsonValue *obj =
            wv_object(db, ids[i], vecs[i], metas ? metas[i] : NULL, uuid);
        if (!obj || !vdb_push(items, obj)) {
          vdb_record_error(db, path, single ? -1 : i,
                           "metadata is nested too deeply");
          ok = 0;
        }
      }
    }
    JsonValue *req = json_object();
    if (ok && del) {
      JsonValue *where = vdb_json_text("{\"path\":[\"id\"],\"operator\":\"ContainsAny\"}");
      vdb_put(where, "valueTextArray", json_retain(items));
      JsonValue *match = json_object();
      BrixString *cls = str_new(db->coll_path);
      vdb_put(match, "class", json_string(cls));
      string_release(cls);
      vdb_put(match, "where", where);
      vdb_put(req, "match", match);
    } else if (ok && !vdb_put_checked(req, "objects", items)) {
      vdb_record_error(db, path, -1, "metadata is nested too deeply");
      ok = 0;
    }
    json_release(items);
    if (!ok) {
      json_release(req);
      break;
    }
    BrixString *body = json_stringify(req);
    json_release(req);
    JsonValue *resp = wv_request(db, del ? "DELETE" : NULL, path, NULL, body->data);
    string_release(body);
    if (!resp) {
      ok = 0;
      break;
    }
    if (del) {
      JsonValue *failed = vdb_take(vdb_field(resp, "results"), "failed");
      if (!failed || failed->tag != JSON_INT) {
        vdb_set_error(db, "%s: response has no \"results.failed\" count", path);
        ok = 0;
      } else if (failed->data.as_int > 0) {
        vdb_set_error(db, "%s: %ld objects failed to delete", path,
                      failed->data.as_int);
        ok = 0;
      } else {
        done += count;
      }
      json_release(failed);
    } else if (resp->tag != JSON_ARRAY || json_array_len(resp) != count) {
      vdb_set_error(db, "%s: response does not list the %ld objects sent", path,
                    count);
      ok = 0;
    } else {
      char msg[400];
      long written = 0;
      for (long i = 0; i < count; i++) {
        if (!wv_item_error(resp, i, msg, sizeof(msg))) {
          written++;
        } else if (ok) {
          vdb_record_error(db, path, single ? -1 : done + i, msg);
          ok = 0;
        }
      }
      done += written;
    }
    json_release(resp);
  }
  if (!ok && n > 1) vdb_note_written(db, done, n);
  return ok;
}

// ---- Grupo B Fase 3: Milvus (REST API v2, /v2/vectordb). One connection is
// one existing collection with a VarChar primary key (Brix ids are strings)
// and one FloatVector field; both field names come from describe on the
// first operation and are cached (any failed request drops them, and stats()
// always describes again). Metadata is the entity's other fields: static
// ones and, when the collection enables it, the dynamic field. Reads use the
// collection's own consistency level. Upsert replaces the whole entity, like
// Pinecone. Filters are Milvus boolean expressions, given as a JSON string
// and sent as-is. A query match has "score": Milvus' "distance", in the
// collection's metric (COSINE / IP: higher is closer; L2: lower). Errors come
// back as HTTP 200 with a non-zero "code" and are not retried (set_retries
// only covers HTTP 429/502/503/504, so neither is Milvus' rate limiting). No
// namespaces: delete_namespace and reset_index are not supported.

#define BRIX_MILVUS_MAX_BATCH 1000

static void mv_forget(BrixVectorDB *db) {
  free(db->coll_path);
  free(db->vec_field);
  db->coll_path = NULL;
  db->vec_field = NULL;
}

// {"collectionName": c[, "dbName": d]} (owned).
static JsonValue *mv_req(BrixVectorDB *db) {
  JsonValue *req = json_object();
  BrixString *c = str_new(db->ns);
  vdb_put(req, "collectionName", json_string(c));
  string_release(c);
  if (db->db_path) {
    BrixString *d = str_new(db->db_path);
    vdb_put(req, "dbName", json_string(d));
    string_release(d);
  }
  return req;
}

// POST `req` (owned) to `path`. Returns "data" (owned; JSON null when
// absent), or NULL with last_error set.
static JsonValue *mv_post(BrixVectorDB *db, const char *path, JsonValue *req) {
  BrixString *body = json_stringify(req);
  json_release(req);
  JsonValue *resp = vdb_request(db, path, NULL, body->data);
  string_release(body);
  if (!resp) {
    mv_forget(db); // the collection may have been dropped or recreated
    return NULL;
  }
  JsonValue *data = vdb_take(resp, "data");
  return data ? data : json_null();
}

static const char *mv_str(JsonValue *v) {
  return v && v->tag == JSON_STRING ? v->data.as_string->data : NULL;
}

// Describe the collection, cache its primary key and vector field names and
// set *dim (-1 when unknown). 0 with last_error set on a failure or an
// unsupported schema.
static int mv_describe(BrixVectorDB *db, long *dim) {
  const char *path = "/v2/vectordb/collections/describe";
  JsonValue *data = mv_post(db, path, mv_req(db));
  if (!data) return 0;
  JsonValue *fields = vdb_take(data, "fields");
  char *pk = NULL, *vec = NULL;
  const char *bad = NULL;
  *dim = -1;
  for (long i = 0; i < json_array_len(fields) && !bad; i++) {
    JsonValue *f = json_index(fields, i);
    JsonValue *name = vdb_field(f, "name");
    JsonValue *type = vdb_field(f, "type");
    JsonValue *primary = vdb_field(f, "primaryKey");
    JsonValue *auto_id = vdb_field(f, "autoId");
    const char *n = mv_str(name), *t = mv_str(type);
    int is_pk = primary && primary->tag == JSON_BOOL && primary->data.as_bool;
    if (!n || !vdb_is_name(n)) {
      if (is_pk || (t && strcmp(t, "FloatVector") == 0)) bad = "a field name is not a name";
    } else if (is_pk) {
      if (!t || strcmp(t, "VarChar") != 0) {
        bad = "the primary key must be a VarChar field (Brix ids are strings)";
      } else if (auto_id && auto_id->tag == JSON_BOOL && auto_id->data.as_bool) {
        bad = "the primary key must not use autoID";
      } else {
        pk = strdup(n);
      }
    } else if (t && strcmp(t, "FloatVector") == 0) {
      if (vec) {
        bad = "the collection must have a single FloatVector field";
      } else {
        vec = strdup(n);
        JsonValue *params = vdb_field(f, "params");
        for (long j = 0; j < json_array_len(params); j++) {
          JsonValue *pp = json_index(params, j);
          JsonValue *key = vdb_field(pp, "key");
          JsonValue *val = vdb_field(pp, "value");
          if (mv_str(key) && strcmp(mv_str(key), "dim") == 0) {
            if (val && val->tag == JSON_INT) *dim = val->data.as_int;
            if (mv_str(val)) *dim = strtol(mv_str(val), NULL, 10);
          }
          json_release(val);
          json_release(key);
          json_release(pp);
        }
        json_release(params);
      }
    }
    json_release(auto_id);
    json_release(primary);
    json_release(type);
    json_release(name);
    json_release(f);
  }
  json_release(fields);
  if (!bad && (!pk || !vec)) {
    bad = pk ? "the collection has no FloatVector field"
             : "the collection has no primary key field";
  }
  if (bad) {
    free(pk);
    free(vec);
    mv_forget(db);
    vdb_set_error(db, "%s: %s", path, bad);
    return 0;
  }
  mv_forget(db);
  db->coll_path = pk;
  db->vec_field = vec;
  return 1;
}

static int mv_resolve(BrixVectorDB *db) {
  long dim;
  return db->coll_path || mv_describe(db, &dim);
}

// The fields of `ent` but the primary key, the vector and nulls; NULL when
// none remain. A search hit (`hit`) also has "distance" and, when the primary
// key has another name, a copy of it under "id".
static JsonValue *mv_meta(BrixVectorDB *db, JsonValue *ent, int hit) {
  if (!ent || ent->tag != JSON_OBJECT) return NULL;
  JsonValue *meta = NULL;
  for (JsonObjectEntry *e = ent->data.as_object->head; e; e = e->next) {
    const char *k = e->key->data;
    if (!e->val || e->val->tag == JSON_NULL || strcmp(k, db->coll_path) == 0 ||
        strcmp(k, db->vec_field) == 0 ||
        (hit && (strcmp(k, "distance") == 0 || strcmp(k, "id") == 0))) {
      continue;
    }
    if (!meta) meta = json_object();
    json_set(meta, e->key, e->val);
  }
  return meta;
}

// `s` as a Milvus string literal: its JSON text (Milvus unquotes the same
// escapes).
static void mv_literal(VdbText *t, BrixString *s) {
  JsonValue *v = json_string(s);
  if (!v) {
    t->oom = 1;
    return;
  }
  vt_json(t, v);
  json_release(v);
}

// stats() -> {"dimension": int, "totalVectorCount": int} (describe +
// count(*)).
static JsonValue *mv_stats(BrixVectorDB *db) {
  long dim;
  if (!mv_describe(db, &dim)) return NULL;
  JsonValue *req = mv_req(db);
  BrixString *empty = str_new("");
  vdb_put(req, "filter", json_string(empty));
  string_release(empty);
  vdb_put(req, "outputFields", vdb_json_text("[\"count(*)\"]"));
  JsonValue *count = vdb_take(
      vdb_take_index(mv_post(db, "/v2/vectordb/entities/query", req), 0),
      "count(*)");
  if (!count || count->tag != JSON_INT) {
    json_release(count);
    if (!db->last_error) {
      vdb_set_error(db, "/v2/vectordb/entities/query: response has no count");
    }
    return NULL;
  }
  JsonValue *out = json_object();
  vdb_put(out, "dimension", dim >= 0 ? json_int(dim) : json_null());
  vdb_put(out, "totalVectorCount", count);
  return out;
}

static JsonValue *mv_query(BrixVectorDB *db, BrixEmbedding *q, long top_k,
                           JsonValue *filter) {
  const char *path = "/v2/vectordb/entities/search";
  if (filter && filter->tag != JSON_STRING) {
    vdb_set_error(db, "%s: filter must be a Milvus filter expression (a JSON "
                  "string)", path);
    return NULL;
  }
  if (!mv_resolve(db)) return NULL;
  JsonValue *req = mv_req(db);
  JsonValue *qs = json_array();
  vdb_push(qs, vdb_embedding_json(q));
  vdb_put(req, "data", qs);
  BrixString *field = str_new(db->vec_field);
  vdb_put(req, "annsField", json_string(field));
  string_release(field);
  vdb_put(req, "limit", json_int(top_k));
  vdb_put(req, "outputFields", vdb_json_text("[\"*\"]"));
  if (filter) vdb_put(req, "filter", json_retain(filter));
  JsonValue *hits = mv_post(db, path, req);
  if (!hits) return NULL;
  if (hits->tag != JSON_ARRAY) {
    json_release(hits);
    vdb_set_error(db, "%s: response has no \"data\" array", path);
    return NULL;
  }
  JsonValue *matches = json_array();
  for (long i = 0; i < json_array_len(hits); i++) {
    JsonValue *hit = json_index(hits, i);
    JsonValue *m = json_object();
    vdb_put(m, "id", vdb_field(hit, db->coll_path));
    vdb_put(m, "score", vdb_field(hit, "distance"));
    vdb_put(m, "metadata", mv_meta(db, hit, 1));
    vdb_push(matches, m);
    json_release(hit);
  }
  json_release(hits);
  return matches;
}

// fetch(id) -> {id, values, metadata}; nil with last_error "" when missing.
static JsonValue *mv_fetch(BrixVectorDB *db, BrixString *id) {
  if (!mv_resolve(db)) return NULL;
  JsonValue *req = mv_req(db);
  JsonValue *ids = json_array();
  vdb_push(ids, json_string(id));
  vdb_put(req, "id", ids);
  JsonValue *out = json_array();
  BrixString *star = str_new("*");
  BrixString *field = str_new(db->vec_field);
  vdb_push(out, json_string(star));
  vdb_push(out, json_string(field));
  string_release(star);
  string_release(field);
  vdb_put(req, "outputFields", out);
  JsonValue *ents = mv_post(db, "/v2/vectordb/entities/get", req);
  if (!ents) return NULL;
  if (ents->tag != JSON_ARRAY) {
    json_release(ents);
    vdb_set_error(db, "/v2/vectordb/entities/get: response has no \"data\" "
                  "array");
    return NULL;
  }
  JsonValue *ent = vdb_take_index(ents, 0);
  if (!ent) return NULL;
  JsonValue *rec = json_object();
  vdb_put(rec, "id", vdb_field(ent, db->coll_path));
  vdb_put(rec, "values", vdb_field(ent, db->vec_field));
  vdb_put(rec, "metadata", mv_meta(db, ent, 0));
  json_release(ent);
  return rec;
}

// list([limit[, token]]) -> {vectors: [{id}], pagination: {next}}: a query
// for ids greater than the token (the last id of the previous page), which
// Milvus returns in primary key order -- a cursor, so no offset cap.
static JsonValue *mv_list(BrixVectorDB *db, long limit, BrixString *token) {
  const char *path = "/v2/vectordb/entities/query";
  if (limit == LONG_MIN) limit = 100; // Pinecone's default page size
  if (limit < 1) {
    vdb_set_error(db, "%s: limit must be >= 1, got %ld", path, limit);
    return NULL;
  }
  if (!mv_resolve(db)) return NULL;
  VdbText f = {0};
  if (token && token->len > 0) {
    vt_add(&f, db->coll_path);
    vt_add(&f, " > ");
    mv_literal(&f, token);
  } else {
    vt_add(&f, "");
  }
  if (f.oom) {
    free(f.data);
    vdb_set_error(db, "%s: out of memory", path);
    return NULL;
  }
  JsonValue *req = mv_req(db);
  BrixString *expr = str_new(f.data);
  free(f.data);
  vdb_put(req, "filter", json_string(expr));
  string_release(expr);
  vdb_put(req, "limit", json_int(limit));
  JsonValue *out = json_array();
  BrixString *pk = str_new(db->coll_path);
  vdb_push(out, json_string(pk));
  string_release(pk);
  vdb_put(req, "outputFields", out);
  JsonValue *ents = mv_post(db, path, req);
  if (!ents) return NULL;
  if (ents->tag != JSON_ARRAY) {
    json_release(ents);
    vdb_set_error(db, "%s: response has no \"data\" array", path);
    return NULL;
  }
  long n = json_array_len(ents);
  JsonValue *vectors = json_array();
  JsonValue *last = NULL;
  for (long i = 0; i < n; i++) {
    json_release(last);
    last = vdb_take(json_index(ents, i), db->coll_path);
    JsonValue *rec = json_object();
    vdb_put(rec, "id", json_retain(last));
    vdb_push(vectors, rec);
  }
  json_release(ents);
  JsonValue *page = json_object();
  vdb_put(page, "vectors", vectors);
  if (n == limit && last && last->tag == JSON_STRING) {
    JsonValue *pag = json_object();
    vdb_put(pag, "next", json_retain(last));
    vdb_put(page, "pagination", pag);
  }
  json_release(last);
  return page;
}

// Upsert item: {<pk>: id, <vector>: values, ...metadata}.
static BrixString *mv_upsert_item(BrixVectorDB *db, void *ctx, long i) {
  VdbUpsertCtx *c = ctx;
  JsonValue *meta = c->metas ? c->metas[i] : NULL;
  JsonValue *ent = json_object();
  size_t want = 2;
  if (meta && meta->tag == JSON_OBJECT) {
    for (JsonObjectEntry *x = meta->data.as_object->head; x; x = x->next) {
      json_set(ent, x->key, x->val); // refuses values nested too deeply
      want++;
    }
  }
  vdb_put(ent, db->coll_path, json_string(c->ids[i]));
  vdb_put(ent, db->vec_field, vdb_embedding_json(c->vecs[i]));
  if (ent->data.as_object->count != want) {
    json_release(ent);
    vdb_record_error(db, "/v2/vectordb/entities/upsert", c->single ? -1 : i,
                     "metadata is nested too deeply");
    return NULL;
  }
  BrixString *text = json_stringify(ent);
  json_release(ent);
  return text;
}

// Upsert (in requests of BRIX_VDB_MAX_BATCH records and BRIX_VDB_MAX_BODY
// bytes; a Milvus request is all or nothing) or delete ("<pk> in [...]", in
// requests of BRIX_MILVUS_MAX_BATCH ids) ids[0..n). A failed request stops
// the batch with "(N of M written)".
static int mv_write(BrixVectorDB *db, int del, long n, BrixString **ids,
                    BrixEmbedding **vecs, JsonValue **metas, int single) {
  const char *path = del ? "/v2/vectordb/entities/delete"
                         : "/v2/vectordb/entities/upsert";
  if (n == 0) return 1;
  if (!mv_resolve(db)) return 0;
  if (!del) {
    for (long i = 0; metas && i < n; i++) {
      // "id" too: a search hit carries the primary key under "id", which
      // would hide a field of that name from query() but not from fetch().
      BrixString *keys[] = {str_new(db->coll_path), str_new(db->vec_field),
                            str_new("id")};
      int clash = 0;
      for (int k = 0; k < 3; k++) {
        JsonValue *v = json_get(metas[i], keys[k]);
        clash |= v != NULL;
        json_release(v);
        string_release(keys[k]);
      }
      if (clash) {
        vdb_record_error(db, path, single ? -1 : i,
                         "metadata must not contain \"id\", the primary key "
                         "or the vector field");
        return 0;
      }
    }
    JsonValue *req = mv_req(db);
    BrixString *text = json_stringify(req);
    json_release(req);
    // `],"collectionName":...}` closes the body after the "data" items.
    char *suffix = vdb_concat("],", text->data + 1, "");
    string_release(text);
    if (!suffix) {
      vdb_set_error(db, "%s: out of memory", path);
      return 0;
    }
    VdbUpsertCtx c = {ids, vecs, metas, single};
    int ok = vdb_post_chunked(db, path, "{\"data\":[", suffix, n,
                              mv_upsert_item, &c);
    free(suffix);
    if (!ok) mv_forget(db);
    return ok;
  }
  long done = 0;
  int ok = 1;
  while (ok && done < n) {
    long count = n - done < BRIX_MILVUS_MAX_BATCH ? n - done
                                                   : BRIX_MILVUS_MAX_BATCH;
    VdbText f = {0};
    vt_add(&f, db->coll_path);
    vt_add(&f, " in [");
    for (long i = done; i < done + count; i++) {
      if (i > done) vt_add(&f, ", ");
      mv_literal(&f, ids[i]);
    }
    vt_add(&f, "]");
    if (f.oom) {
      free(f.data);
      vdb_set_error(db, "%s: out of memory", path);
      ok = 0;
      break;
    }
    JsonValue *req = mv_req(db);
    BrixString *expr = str_new(f.data);
    free(f.data);
    vdb_put(req, "filter", json_string(expr));
    string_release(expr);
    JsonValue *resp = mv_post(db, path, req);
    if (!resp) {
      ok = 0;
      break;
    }
    json_release(resp);
    done += count;
  }
  if (!ok && n > 1) vdb_note_written(db, done, n);
  return ok;
}

// db.last_error() -> string ("" when the last operation succeeded).
BrixString *brix_vectordb_last_error(BrixVectorDB *db) {
  return str_new(db->last_error);
}

// ==========================================
// SECTION 2.13: ONNX MODELS (v2.2 Grupo A)
// ==========================================
//
// onnx.load(path) opens a model with the ONNX Runtime C API on the CPU.
// libonnxruntime is loaded with dlopen on the first load, so a program that
// never calls onnx.load does not need it installed (nothing links
// -lonnxruntime). Its only exported symbol, OrtGetApiBase, hands out the
// OrtApi function table; the ORT ABI guarantees that table is append-only and
// never reordered, so the few slots used here are called by index instead of
// vendoring the 9k-line header. Indices come from offsetof(OrtApi, X) on
// onnxruntime_c_api.h 1.31; every slot used exists since API version 1,
// which is the version requested (so older installs work too).

#include <dlfcn.h>

#define BRIX_ORT_API_VERSION 1

enum {
  ORT_GET_ERROR_MESSAGE = 2,
  ORT_CREATE_ENV = 3,
  ORT_CREATE_SESSION = 7,
  ORT_RUN = 9,
  ORT_CREATE_SESSION_OPTIONS = 10,
  ORT_SESSION_GET_INPUT_COUNT = 30,
  ORT_SESSION_GET_OUTPUT_COUNT = 31,
  ORT_SESSION_GET_INPUT_TYPE_INFO = 33,
  ORT_SESSION_GET_OUTPUT_TYPE_INFO = 34,
  ORT_SESSION_GET_INPUT_NAME = 36,
  ORT_SESSION_GET_OUTPUT_NAME = 37,
  ORT_CREATE_TENSOR_AS_ORT_VALUE = 48,
  ORT_GET_TENSOR_MUTABLE_DATA = 51,
  ORT_CAST_TYPE_INFO_TO_TENSOR_INFO = 55,
  ORT_GET_TENSOR_ELEMENT_TYPE = 60,
  ORT_GET_DIMENSIONS_COUNT = 61,
  ORT_GET_DIMENSIONS = 62,
  ORT_GET_TENSOR_TYPE_AND_SHAPE = 65,
  ORT_ALLOCATOR_FREE = 76,
  ORT_GET_ALLOCATOR_WITH_DEFAULT_OPTIONS = 78,
  ORT_RELEASE_SESSION = 95,
  ORT_RELEASE_VALUE = 96,
  ORT_RELEASE_TYPE_INFO = 98,
  ORT_RELEASE_TENSOR_TYPE_AND_SHAPE_INFO = 99,
  ORT_RELEASE_SESSION_OPTIONS = 100,
};
#define BRIX_ORT_LOGGING_LEVEL_ERROR 3
// ONNXTensorElementDataType values Brix converts to/from f64.
enum { ORT_FLOAT = 1, ORT_INT32 = 6, ORT_INT64 = 7, ORT_DOUBLE = 11 };

typedef void OrtStatusPtr; // OrtStatus*: NULL = success
typedef struct {
  const void *(*GetApi)(uint32_t version);
  const char *(*GetVersionString)(void);
} BrixOrtApiBase;

static void *const *brix_ort; // the OrtApi table, NULL until loaded
static void *brix_ort_env;    // OrtEnv*, shared by every model

#define ORT_FN(slot, type) ((type)brix_ort[slot])

typedef struct {
  long ref_count;
  void *session;   // OrtSession*
  long num_inputs;
  long num_outputs;
  char **names;    // inputs, then outputs
  long **shapes;   // per input/output; -1 = a dynamic dimension, NULL = not a tensor
  long *ranks;
  int *types;      // per input/output: ONNXTensorElementDataType (0 = not a tensor)
} BrixOnnxModel;

static void onnx_fail(const char *path, const char *msg) {
  fprintf(stderr, "Error: onnx.load(\"%s\"): %s\n", path, msg);
  exit(1);
}

// Abort with the message of a failed ORT call.
static void onnx_check(const char *path, OrtStatusPtr *st) {
  if (!st) return;
  const char *msg = ORT_FN(ORT_GET_ERROR_MESSAGE, const char *(*)(OrtStatusPtr *))(st);
  onnx_fail(path, msg ? msg : "ONNX Runtime error");
}

// dlopen libonnxruntime and create the shared OrtEnv, once.
static void onnx_init(const char *path) {
  if (brix_ort) return;
  const char *env = getenv("BRIX_ONNXRUNTIME_LIB");
  // ponytail: fixed search list (env; Linux: the loader's path; macOS:
  // Homebrew); add a
  // config option if installs elsewhere get common.
  const char *candidates[] = {
#ifdef __APPLE__
      // Absolute paths only: for a bare name, macOS dlopen also searches the
      // current directory, so a planted library there would be loaded.
      "/opt/homebrew/lib/libonnxruntime.dylib",
      "/usr/local/lib/libonnxruntime.dylib",
#else
      "libonnxruntime.so", "libonnxruntime.so.1",
#endif
  };
  void *lib = NULL;
  if (env && *env) {
    lib = dlopen(env, RTLD_NOW | RTLD_LOCAL);
  } else {
    for (size_t i = 0; !lib && i < sizeof(candidates) / sizeof(candidates[0]); i++) {
      lib = dlopen(candidates[i], RTLD_NOW | RTLD_LOCAL);
    }
  }
  if (!lib) {
    char msg[600];
    snprintf(msg, sizeof(msg),
             "ONNX Runtime not found (%s); install it (macOS: brew install "
             "onnxruntime; Linux: the onnxruntime release from GitHub) or set "
             "BRIX_ONNXRUNTIME_LIB to the path of libonnxruntime",
             dlerror());
    onnx_fail(path, msg);
  }
  const BrixOrtApiBase *(*get_base)(void) =
      (const BrixOrtApiBase *(*)(void))dlsym(lib, "OrtGetApiBase");
  const BrixOrtApiBase *base = get_base ? get_base() : NULL;
  void *const *api = base ? (void *const *)base->GetApi(BRIX_ORT_API_VERSION) : NULL;
  if (!api) onnx_fail(path, "the library found is not a usable ONNX Runtime");
  brix_ort = api;
  onnx_check(path, ORT_FN(ORT_CREATE_ENV, OrtStatusPtr *(*)(int, const char *, void **))(
                       BRIX_ORT_LOGGING_LEVEL_ERROR, "brix", &brix_ort_env));
}

// Name and shape of input/output `i` (`output` selects which), copied into
// the model.
static void onnx_read_io(BrixOnnxModel *m, const char *path, void *alloc,
                         int output, long i) {
  long slot = output ? m->num_inputs + i : i;
  char *name = NULL;
  onnx_check(path, ORT_FN(output ? ORT_SESSION_GET_OUTPUT_NAME : ORT_SESSION_GET_INPUT_NAME,
                          OrtStatusPtr *(*)(void *, size_t, void *, char **))(
                       m->session, (size_t)i, alloc, &name));
  m->names[slot] = strdup(name);
  ORT_FN(ORT_ALLOCATOR_FREE, OrtStatusPtr *(*)(void *, void *))(alloc, name);

  void *type_info = NULL;
  onnx_check(path, ORT_FN(output ? ORT_SESSION_GET_OUTPUT_TYPE_INFO
                                 : ORT_SESSION_GET_INPUT_TYPE_INFO,
                          OrtStatusPtr *(*)(void *, size_t, void **))(
                       m->session, (size_t)i, &type_info));
  const void *tensor = NULL; // NULL when the value is not a tensor
  onnx_check(path, ORT_FN(ORT_CAST_TYPE_INFO_TO_TENSOR_INFO,
                          OrtStatusPtr *(*)(const void *, const void **))(type_info, &tensor));
  if (tensor) {
    size_t rank = 0;
    onnx_check(path, ORT_FN(ORT_GET_DIMENSIONS_COUNT, OrtStatusPtr *(*)(const void *, size_t *))(
                         tensor, &rank));
    int64_t *dims = calloc(rank ? rank : 1, sizeof(int64_t));
    m->shapes[slot] = calloc(rank ? rank : 1, sizeof(long));
    if (!dims || !m->shapes[slot]) onnx_fail(path, "out of memory");
    onnx_check(path, ORT_FN(ORT_GET_DIMENSIONS,
                            OrtStatusPtr *(*)(const void *, int64_t *, size_t))(tensor, dims, rank));
    for (size_t d = 0; d < rank; d++) m->shapes[slot][d] = dims[d] < 0 ? -1 : (long)dims[d];
    m->ranks[slot] = (long)rank;
    free(dims);
    int type = 0;
    onnx_check(path, ORT_FN(ORT_GET_TENSOR_ELEMENT_TYPE, OrtStatusPtr *(*)(const void *, int *))(
                         tensor, &type));
    m->types[slot] = type;
  }
  ORT_FN(ORT_RELEASE_TYPE_INFO, void (*)(void *))(type_info);
  if (!m->names[slot]) onnx_fail(path, "out of memory");
}

// onnx.load(path) -> ONNXModel. Aborts when ONNX Runtime is missing or the
// model can't be loaded (the message is ONNX Runtime's).
BrixOnnxModel *brix_onnx_load(BrixString *path_str) {
  const char *path = path_str ? path_str->data : "";
  onnx_init(path);
  void *opts = NULL;
  onnx_check(path, ORT_FN(ORT_CREATE_SESSION_OPTIONS, OrtStatusPtr *(*)(void **))(&opts));
  BrixOnnxModel *m = calloc(1, sizeof(BrixOnnxModel));
  if (!m) onnx_fail(path, "out of memory");
  m->ref_count = 1;
  OrtStatusPtr *st = ORT_FN(ORT_CREATE_SESSION,
                            OrtStatusPtr *(*)(const void *, const char *, const void *, void **))(
      brix_ort_env, path, opts, &m->session);
  ORT_FN(ORT_RELEASE_SESSION_OPTIONS, void (*)(void *))(opts);
  onnx_check(path, st);

  size_t n_in = 0, n_out = 0;
  onnx_check(path, ORT_FN(ORT_SESSION_GET_INPUT_COUNT, OrtStatusPtr *(*)(const void *, size_t *))(
                       m->session, &n_in));
  onnx_check(path, ORT_FN(ORT_SESSION_GET_OUTPUT_COUNT, OrtStatusPtr *(*)(const void *, size_t *))(
                       m->session, &n_out));
  m->num_inputs = (long)n_in;
  m->num_outputs = (long)n_out;
  size_t n = n_in + n_out;
  m->names = calloc(n ? n : 1, sizeof(char *));
  m->shapes = calloc(n ? n : 1, sizeof(long *));
  m->ranks = calloc(n ? n : 1, sizeof(long));
  m->types = calloc(n ? n : 1, sizeof(int));
  if (!m->names || !m->shapes || !m->ranks || !m->types) onnx_fail(path, "out of memory");
  void *alloc = NULL;
  onnx_check(path, ORT_FN(ORT_GET_ALLOCATOR_WITH_DEFAULT_OPTIONS, OrtStatusPtr *(*)(void **))(&alloc));
  for (long i = 0; i < m->num_inputs; i++) onnx_read_io(m, path, alloc, 0, i);
  for (long i = 0; i < m->num_outputs; i++) onnx_read_io(m, path, alloc, 1, i);
  return m;
}

BrixOnnxModel *brix_onnx_retain(BrixOnnxModel *m) {
  if (m) m->ref_count++;
  return m;
}

void brix_onnx_release(BrixOnnxModel *m) {
  if (!m || --m->ref_count > 0) return;
  ORT_FN(ORT_RELEASE_SESSION, void (*)(void *))(m->session);
  for (long i = 0; i < m->num_inputs + m->num_outputs; i++) {
    free(m->names[i]);
    free(m->shapes[i]);
  }
  free(m->names);
  free(m->shapes);
  free(m->ranks);
  free(m->types);
  free(m);
}

long brix_onnx_num_inputs(BrixOnnxModel *m) { return m->num_inputs; }
long brix_onnx_num_outputs(BrixOnnxModel *m) { return m->num_outputs; }

// Slot of input/output `i`; aborts when out of range.
static long onnx_slot(BrixOnnxModel *m, const char *method, int output, long i) {
  long count = output ? m->num_outputs : m->num_inputs;
  if (i < 0 || i >= count) {
    fprintf(stderr, "Error: ONNXModel.%s(%ld): index out of range (%ld %s)\n",
            method, i, count, output ? "outputs" : "inputs");
    exit(1);
  }
  return output ? m->num_inputs + i : i;
}

static BrixString *onnx_name(BrixOnnxModel *m, const char *method, int output, long i) {
  return str_new(m->names[onnx_slot(m, method, output, i)]);
}

// 1×rank IntMatrix; -1 marks a dynamic dimension (a scalar is 1×0).
static IntMatrix *onnx_shape(BrixOnnxModel *m, const char *method, int output, long i) {
  long slot = onnx_slot(m, method, output, i);
  if (!m->shapes[slot]) {
    fprintf(stderr, "Error: ONNXModel.%s(%ld): %s %ld is not a tensor\n", method,
            i, output ? "output" : "input", i);
    exit(1);
  }
  IntMatrix *out = intmatrix_new(1, m->ranks[slot]);
  for (long d = 0; d < m->ranks[slot]; d++) out->data[d] = m->shapes[slot][d];
  return out;
}

BrixString *brix_onnx_input_name(BrixOnnxModel *m, long i) {
  return onnx_name(m, "input_name", 0, i);
}
BrixString *brix_onnx_output_name(BrixOnnxModel *m, long i) {
  return onnx_name(m, "output_name", 1, i);
}
IntMatrix *brix_onnx_input_shape(BrixOnnxModel *m, long i) {
  return onnx_shape(m, "input_shape", 0, i);
}
IntMatrix *brix_onnx_output_shape(BrixOnnxModel *m, long i) {
  return onnx_shape(m, "output_shape", 1, i);
}

// --- Inference (Fase 2) ---------------------------------------------------
// model.infer(x, ...) / model.infer_output(out, x, ...): one Matrix per model
// input, converted to the input's element type (float, double, int32, int64);
// one output, converted back to a f64 Matrix. Every error aborts (graceful
// handling is Fase 4).

static void onnx_infer_fail(const char *method, const char *msg) {
  fprintf(stderr, "Error: ONNXModel.%s: %s\n", method, msg);
  exit(1);
}

static void onnx_infer_check(const char *method, OrtStatusPtr *st) {
  if (!st) return;
  const char *msg = ORT_FN(ORT_GET_ERROR_MESSAGE, const char *(*)(OrtStatusPtr *))(st);
  onnx_infer_fail(method, msg ? msg : "ONNX Runtime error");
}

// "[-1, 3]" into buf.
static void onnx_fmt_shape(char *buf, size_t size, const long *dims, long rank) {
  size_t used = (size_t)snprintf(buf, size, "[");
  for (long d = 0; d < rank && used < size; d++) {
    used += (size_t)snprintf(buf + used, size - used, "%s%ld", d ? ", " : "", dims[d]);
  }
  if (used < size) snprintf(buf + used, size - used, "]");
}

static size_t onnx_elem_size(int type) {
  switch (type) {
  case ORT_FLOAT:
  case ORT_INT32:
    return 4;
  case ORT_INT64:
  case ORT_DOUBLE:
    return 8;
  default:
    return 0;
  }
}

// Abort unless input/output `slot` is a tensor of a supported element type.
static void onnx_check_type(BrixOnnxModel *m, const char *method, long slot) {
  int output = slot >= m->num_inputs;
  long i = output ? slot - m->num_inputs : slot;
  char msg[512];
  if (!m->shapes[slot]) {
    snprintf(msg, sizeof(msg), "%s %ld (\"%s\") is not a tensor", output ? "output" : "input",
             i, m->names[slot]);
    onnx_infer_fail(method, msg);
  }
  if (!onnx_elem_size(m->types[slot])) {
    snprintf(msg, sizeof(msg),
             "%s %ld (\"%s\") has element type %d; only float, double, int32 and "
             "int64 tensors are supported",
             output ? "output" : "input", i, m->names[slot], m->types[slot]);
    onnx_infer_fail(method, msg);
  }
}

// Concrete shape of input `i` for `x`: a rank-2 input takes the matrix's
// rows x cols; any other rank is filled in order from the element count, with
// at most one dynamic dimension deduced from it. Aborts when `x` doesn't fit.
static void onnx_input_dims(BrixOnnxModel *m, const char *method, long i, Matrix *x,
                            int64_t *dims) {
  const long *want = m->shapes[i];
  long rank = m->ranks[i];
  long total = x->rows * x->cols;
  int ok = 1;
  if (rank == 2) {
    dims[0] = x->rows;
    dims[1] = x->cols;
    for (long d = 0; d < 2; d++) ok &= want[d] < 0 || want[d] == dims[d];
  } else {
    long fixed = 1, dynamic = -1, n_dynamic = 0;
    for (long d = 0; d < rank; d++) {
      if (want[d] < 0) {
        dynamic = d;
        n_dynamic++;
      } else {
        fixed *= want[d];
      }
      dims[d] = want[d];
    }
    if (n_dynamic > 1) {
      ok = 0;
    } else if (n_dynamic == 1) {
      ok = fixed > 0 ? total % fixed == 0 : total == 0;
      dims[dynamic] = fixed > 0 ? total / fixed : 0;
    } else {
      ok = fixed == total;
    }
  }
  if (ok) return;
  char shape[256], msg[512];
  onnx_fmt_shape(shape, sizeof(shape), want, rank);
  snprintf(msg, sizeof(msg),
           "input %ld (\"%s\") expects shape %s, got a %ldx%ld matrix%s", i, m->names[i],
           shape, x->rows, x->cols,
           rank != 2 && total > 0 ? " (its elements fill the shape in order; "
                                    "at most one dimension can be dynamic)"
                                  : "");
  onnx_infer_fail(method, msg);
}

// Copy a Matrix into a fresh tensor of input `i`'s element type, with the
// shape `dims` (NULL: worked out from the matrix by onnx_input_dims).
static void *onnx_make_input(BrixOnnxModel *m, const char *method, void *alloc, long i,
                             Matrix *x, const int64_t *dims) {
  onnx_check_type(m, method, i);
  long rank = m->ranks[i];
  int64_t *own = NULL;
  if (!dims) {
    own = calloc(rank ? rank : 1, sizeof(int64_t));
    if (!own) onnx_infer_fail(method, "out of memory");
    onnx_input_dims(m, method, i, x, own);
    dims = own;
  }
  void *value = NULL;
  onnx_infer_check(method, ORT_FN(ORT_CREATE_TENSOR_AS_ORT_VALUE,
                                  OrtStatusPtr *(*)(void *, const int64_t *, size_t, int, void **))(
                               alloc, dims, (size_t)rank, m->types[i], &value));
  free(own);
  void *data = NULL;
  onnx_infer_check(method, ORT_FN(ORT_GET_TENSOR_MUTABLE_DATA, OrtStatusPtr *(*)(void *, void **))(
                               value, &data));
  long total = x->rows * x->cols;
  int type = m->types[i];
  for (long k = 0; k < total; k++) {
    double v = x->data[k];
    // Integer inputs truncate toward zero (1.9 -> 1); a value the integer
    // type can't hold (NaN, Inf, out of range) aborts: converting it is
    // undefined behavior in C.
    if ((type == ORT_INT32 && !(v > -2147483649.0 && v < 2147483648.0)) ||
        (type == ORT_INT64 && !(v >= -9223372036854775808.0 && v < 9223372036854775808.0))) {
      char msg[512];
      snprintf(msg, sizeof(msg), "input %ld (\"%s\") element %ld (%g) does not fit in an %s", i,
               m->names[i], k, v, type == ORT_INT32 ? "int32" : "int64");
      onnx_infer_fail(method, msg);
    }
    switch (type) {
    case ORT_FLOAT: ((float *)data)[k] = (float)v; break;
    case ORT_DOUBLE: ((double *)data)[k] = v; break;
    case ORT_INT32: ((int32_t *)data)[k] = (int32_t)v; break;
    default: ((int64_t *)data)[k] = (int64_t)v; break;
    }
  }
  return value;
}

// Convert an output tensor to a Matrix: a scalar is 1x1, [n] is 1xn, [r, c]
// is r x c and [d0, d1, ...] is d0 x (d1*d2*...). With `batch` >= 0 the
// first dimension must be `batch`, and each sample becomes one row ([n] is
// then n x 1).
static Matrix *onnx_output_matrix(const char *method, void *value, long batch) {
  void *info = NULL;
  onnx_infer_check(method, ORT_FN(ORT_GET_TENSOR_TYPE_AND_SHAPE,
                                  OrtStatusPtr *(*)(const void *, void **))(value, &info));
  int type = 0;
  size_t rank = 0;
  onnx_infer_check(method, ORT_FN(ORT_GET_TENSOR_ELEMENT_TYPE,
                                  OrtStatusPtr *(*)(const void *, int *))(info, &type));
  onnx_infer_check(method, ORT_FN(ORT_GET_DIMENSIONS_COUNT,
                                  OrtStatusPtr *(*)(const void *, size_t *))(info, &rank));
  int64_t *dims = calloc(rank ? rank : 1, sizeof(int64_t));
  if (!dims) onnx_infer_fail(method, "out of memory");
  onnx_infer_check(method, ORT_FN(ORT_GET_DIMENSIONS,
                                  OrtStatusPtr *(*)(const void *, int64_t *, size_t))(info, dims,
                                                                                      rank));
  ORT_FN(ORT_RELEASE_TENSOR_TYPE_AND_SHAPE_INFO, void (*)(void *))(info);
  if (!onnx_elem_size(type)) {
    char msg[128];
    snprintf(msg, sizeof(msg), "the output has element type %d; only float, double, int32 "
                               "and int64 tensors are supported", type);
    onnx_infer_fail(method, msg);
  }
  long rows = 1, cols = 1;
  if (batch >= 0) {
    if (rank == 0 || dims[0] != batch) {
      char msg[256];
      snprintf(msg, sizeof(msg),
               "the output has no batch dimension (its first dimension is %ld, the batch "
               "has %ld rows)",
               rank ? (long)dims[0] : 0L, batch);
      onnx_infer_fail(method, msg);
    }
    rows = batch;
    for (size_t d = 1; d < rank; d++) cols *= (long)dims[d];
  } else if (rank == 1) {
    cols = (long)dims[0];
  } else if (rank >= 2) {
    rows = (long)dims[0];
    for (size_t d = 1; d < rank; d++) cols *= (long)dims[d];
  }
  free(dims);
  void *data = NULL;
  onnx_infer_check(method, ORT_FN(ORT_GET_TENSOR_MUTABLE_DATA, OrtStatusPtr *(*)(void *, void **))(
                               value, &data));
  Matrix *out = matrix_new(rows, cols);
  long total = rows * cols;
  for (long k = 0; k < total; k++) {
    switch (type) {
    case ORT_FLOAT: out->data[k] = ((float *)data)[k]; break;
    case ORT_DOUBLE: out->data[k] = ((double *)data)[k]; break;
    case ORT_INT32: out->data[k] = ((int32_t *)data)[k]; break;
    default: out->data[k] = (double)((int64_t *)data)[k]; break;
    }
  }
  return out;
}

// Index of the output named `name`; aborts when there is none. `batch` only
// picks the method name used in the error message.
long brix_onnx_output_index(BrixOnnxModel *m, BrixString *name, long batch) {
  const char *want = name ? name->data : "";
  for (long i = 0; i < m->num_outputs; i++) {
    if (strcmp(m->names[m->num_inputs + i], want) == 0) return i;
  }
  char msg[512];
  snprintf(msg, sizeof(msg), "the model has no output named \"%s\"", want);
  onnx_infer_fail(batch ? "batch_infer_output" : "infer_output", msg);
  return -1;
}

// Abort unless there is one input per model input and output `output`
// exists and is a supported tensor; returns that output's slot.
static long onnx_check_call(BrixOnnxModel *m, const char *method, long n, long output) {
  if (n != m->num_inputs) {
    char names[256] = "", msg[512];
    size_t used = 0;
    for (long i = 0; i < m->num_inputs && used < sizeof(names); i++) {
      used += (size_t)snprintf(names + used, sizeof(names) - used, "%s%s", i ? ", " : "",
                               m->names[i]);
    }
    snprintf(msg, sizeof(msg), "the model takes %ld input(s) (%s), got %ld", m->num_inputs,
             names, n);
    onnx_infer_fail(method, msg);
  }
  long out_slot = onnx_slot(m, method, 1, output);
  onnx_check_type(m, method, out_slot);
  return out_slot;
}

static void *onnx_allocator(const char *method) {
  void *alloc = NULL;
  onnx_infer_check(method, ORT_FN(ORT_GET_ALLOCATOR_WITH_DEFAULT_OPTIONS,
                                  OrtStatusPtr *(*)(void **))(&alloc));
  return alloc;
}

// One Run on the `n` input tensors; returns output `out_slot` as a Matrix
// (see onnx_output_matrix for `batch`).
static Matrix *onnx_run(BrixOnnxModel *m, const char *method, void **values, long n,
                        long out_slot, long batch) {
  void *result = NULL;
  const char *out_name = m->names[out_slot];
  onnx_infer_check(method, ORT_FN(ORT_RUN, OrtStatusPtr *(*)(void *, const void *,
                                                             const char *const *,
                                                             const void *const *, size_t,
                                                             const char *const *, size_t,
                                                             void **))(
                               m->session, NULL, (const char *const *)m->names,
                               (const void *const *)values, (size_t)n, &out_name, 1,
                               &result));
  Matrix *out = onnx_output_matrix(method, result, batch);
  ORT_FN(ORT_RELEASE_VALUE, void (*)(void *))(result);
  return out;
}

static void onnx_release_values(void **values, long n) {
  for (long i = 0; i < n; i++) {
    if (values[i]) ORT_FN(ORT_RELEASE_VALUE, void (*)(void *))(values[i]);
  }
  free(values);
}

// Run the model on `n` inputs (one Matrix per model input, in order) and
// return output `output` as a Matrix (owned). `via_infer_output` only picks
// the method name used in error messages.
Matrix *brix_onnx_infer(BrixOnnxModel *m, Matrix **inputs, long n, long output,
                        long via_infer_output) {
  const char *method = via_infer_output ? "infer_output" : "infer";
  long out_slot = onnx_check_call(m, method, n, output);
  void *alloc = onnx_allocator(method);
  void **values = calloc(n ? n : 1, sizeof(void *));
  if (!values) onnx_infer_fail(method, "out of memory");
  for (long i = 0; i < n; i++) values[i] = onnx_make_input(m, method, alloc, i, inputs[i], NULL);
  Matrix *out = onnx_run(m, method, values, n, out_slot, -1);
  onnx_release_values(values, n);
  return out;
}

// model.batch_infer(batch, ...) / batch_infer_output(out, batch, ...): input
// 0 is a batch with one sample per row; each row fills the input's shape
// after its first (batch) dimension. A dynamic batch dimension takes every
// row in one Run; a fixed one (b) runs the rows in chunks of b, zero-padding
// the last chunk and dropping the padded rows' outputs. The other inputs are
// passed unchanged to every Run. Returns rows x (output elements per sample).
Matrix *brix_onnx_batch_infer(BrixOnnxModel *m, Matrix **inputs, long n, long output,
                              long via_output) {
  const char *method = via_output ? "batch_infer_output" : "batch_infer";
  long out_slot = onnx_check_call(m, method, n, output);
  onnx_check_type(m, method, 0);
  long rank = m->ranks[0];
  const long *want = m->shapes[0];
  char shape[256], msg[600];
  onnx_fmt_shape(shape, sizeof(shape), want, rank);
  if (rank == 0) {
    snprintf(msg, sizeof(msg), "input 0 (\"%s\") has no batch dimension (shape %s)",
             m->names[0], shape);
    onnx_infer_fail(method, msg);
  }
  Matrix *batch = inputs[0];
  long rows = batch->rows, per_sample = batch->cols;

  // Shape of one chunk: dims[0] = chunk rows, the rest filled from a row,
  // with at most one dynamic dimension deduced from its length.
  int64_t *dims = calloc((size_t)rank, sizeof(int64_t));
  if (!dims) onnx_infer_fail(method, "out of memory");
  long fixed = 1, dynamic = -1, n_dynamic = 0;
  for (long d = 1; d < rank; d++) {
    dims[d] = want[d];
    if (want[d] < 0) {
      dynamic = d;
      n_dynamic++;
    } else {
      fixed *= want[d];
    }
  }
  int ok = n_dynamic == 0   ? fixed == per_sample
           : n_dynamic == 1 ? (fixed > 0 ? per_sample % fixed == 0 : per_sample == 0)
                            : 0;
  if (!ok) {
    char sample[256];
    onnx_fmt_shape(sample, sizeof(sample), want + 1, rank - 1);
    snprintf(msg, sizeof(msg),
             "input 0 (\"%s\", shape %s) expects samples of shape %s, got rows of %ld "
             "element(s) (each row fills one sample in order; at most one dimension can be "
             "dynamic)",
             m->names[0], shape, sample, per_sample);
    onnx_infer_fail(method, msg);
  }
  // An empty batch runs nothing, but its width is still checked above.
  if (rows == 0) {
    free(dims);
    return matrix_new(0, 0);
  }
  if (n_dynamic == 1) dims[dynamic] = fixed > 0 ? per_sample / fixed : 0;
  long chunk = want[0] < 0 ? rows : want[0];
  if (chunk <= 0) {
    snprintf(msg, sizeof(msg), "input 0 (\"%s\") has a batch dimension of %ld (shape %s)",
             m->names[0], chunk, shape);
    onnx_infer_fail(method, msg);
  }
  dims[0] = chunk;

  void *alloc = onnx_allocator(method);
  void **values = calloc((size_t)n, sizeof(void *));
  if (!values) onnx_infer_fail(method, "out of memory");
  for (long i = 1; i < n; i++) values[i] = onnx_make_input(m, method, alloc, i, inputs[i], NULL);

  Matrix *rows_in = matrix_new(chunk, per_sample); // calloc: padding rows stay zero
  Matrix *out = NULL;
  for (long start = 0; start < rows; start += chunk) {
    long take = rows - start < chunk ? rows - start : chunk;
    memcpy(rows_in->data, batch->data + start * per_sample,
           (size_t)(take * per_sample) * sizeof(double));
    if (take < chunk) {
      memset(rows_in->data + take * per_sample, 0,
             (size_t)((chunk - take) * per_sample) * sizeof(double));
    }
    values[0] = onnx_make_input(m, method, alloc, 0, rows_in, dims);
    Matrix *part = onnx_run(m, method, values, n, out_slot, chunk);
    ORT_FN(ORT_RELEASE_VALUE, void (*)(void *))(values[0]);
    values[0] = NULL;
    if (!out) out = matrix_new(rows, part->cols);
    if (part->cols != out->cols) {
      onnx_infer_fail(method, "the output size per sample changed between chunks");
    }
    memcpy(out->data + start * out->cols, part->data,
           (size_t)(take * out->cols) * sizeof(double));
    matrix_release(part);
  }
  matrix_release(rows_in);
  onnx_release_values(values, n);
  free(dims);
  return out;
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

// math.stddev alias for brix_std
double brix_stddev(Matrix *m) { return brix_std(m); }

// Math utility wrappers (brix_ prefix avoids LLVM treating fabs/fmin/fmax as intrinsics)
double brix_abs(double x) { return fabs(x); }
double brix_min(double a, double b) { return fmin(a, b); }
double brix_max(double a, double b) { return fmax(a, b); }
double brix_mod(double a, double b) { return fmod(a, b); }

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

// ones(rows, cols) — Matrix filled with 1.0
Matrix *brix_ones(long rows, long cols) {
  Matrix *m = matrix_new(rows, cols);
  long size = rows * cols;
  for (long i = 0; i < size; i++) {
    m->data[i] = 1.0;
  }
  return m;
}

// linspace(start, stop, n) — n evenly spaced points from start to stop (inclusive)
Matrix *brix_linspace(double start, double stop, long n) {
  if (n <= 0) n = 1;
  Matrix *m = matrix_new(1, n);
  if (n == 1) {
    m->data[0] = start;
    return m;
  }
  double step = (stop - start) / (double)(n - 1);
  for (long i = 0; i < n; i++) {
    m->data[i] = start + step * (double)i;
  }
  return m;
}

// arange(start, stop, step) — values from start up to (but not including) stop, with given step
Matrix *brix_arange(double start, double stop, double step) {
  if (step == 0.0) step = 1.0;
  long n = 0;
  if (step > 0.0) {
    for (double v = start; v < stop; v += step) n++;
  } else {
    for (double v = start; v > stop; v += step) n++;
  }
  if (n <= 0) n = 1;
  Matrix *m = matrix_new(1, n);
  double v = start;
  for (long i = 0; i < n; i++) {
    m->data[i] = v;
    v += step;
  }
  return m;
}

// Seed rand() once at program startup
__attribute__((constructor)) static void brix_seed_rng(void) {
  srand((unsigned int)time(NULL));
}

// rand_matrix(rows, cols) — Matrix with random floats in [0.0, 1.0)
Matrix *brix_rand_matrix(long rows, long cols) {
  Matrix *m = matrix_new(rows, cols);
  long size = rows * cols;
  for (long i = 0; i < size; i++) {
    m->data[i] = (double)rand() / ((double)RAND_MAX + 1.0);
  }
  return m;
}

// irand_matrix(n, max_val) — IntMatrix 1×n with random ints in [0, max_val)
IntMatrix *brix_irand_matrix(long n, long max_val) {
  if (max_val <= 0) max_val = 1;
  IntMatrix *m = intmatrix_new(1, n);
  for (long i = 0; i < n; i++) {
    m->data[i] = (long)(rand() % (int)max_val);
  }
  return m;
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

// ==========================================
// SECTION 8: TEST LIBRARY (v1.5)
// Jest-style testing framework
// ==========================================

// ANSI color codes
#define ANSI_RED     "\x1b[31m"
#define ANSI_GREEN   "\x1b[32m"
#define ANSI_YELLOW  "\x1b[33m"
#define ANSI_GRAY    "\x1b[90m"
#define ANSI_BOLD    "\x1b[1m"
#define ANSI_RESET   "\x1b[0m"

#define BRIX_MAX_TESTS  1024
#define BRIX_MAX_HOOKS  32
#define BRIX_ERR_BUF    2048

// Closure type: fn_ptr(env_ptr)
typedef void (*BrixClosureVoid)(void*);

// Hook entry
typedef struct {
    void* fn_ptr;
    void* env_ptr;
} BrixHook;

// Individual test entry
typedef struct {
    char*  name;
    void*  fn_ptr;
    void*  env_ptr;
    int    is_async;
    void*  state_ptr;
    BrixPollResult (*poll_fn)(void*);
    int    passed;
    double duration_ms;
    char   error_msg[BRIX_ERR_BUF];
    char   file[512];
    int    line;
} BrixTestEntry;

// Test suite (one per describe block)
typedef struct {
    char*         suite_name;
    BrixTestEntry tests[BRIX_MAX_TESTS];
    int           test_count;
    int           passed_count;
    int           failed_count;

    BrixHook before_all[BRIX_MAX_HOOKS];   int before_all_count;
    BrixHook after_all[BRIX_MAX_HOOKS];    int after_all_count;
    BrixHook before_each[BRIX_MAX_HOOKS];  int before_each_count;
    BrixHook after_each[BRIX_MAX_HOOKS];   int after_each_count;
} BrixTestSuite;

// Global state
static BrixTestSuite* g_suite            = NULL;
static jmp_buf        g_test_jmp;
static int            g_test_failed      = 0;
static int            g_current_test_idx = -1;

// ---- Timer ----
static double brix_now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000.0 + ts.tv_nsec / 1e6;
}

// ---- Run a void closure ----
static void brix_call_void(void* fn_ptr, void* env_ptr) {
    ((BrixClosureVoid)fn_ptr)(env_ptr);
}

// ---- Get current running test ----
static BrixTestEntry* brix_get_current(void) {
    if (!g_suite || g_current_test_idx < 0) return NULL;
    return &g_suite->tests[g_current_test_idx];
}

// ---- Record failure and jump back to runner ----
static void brix_test_fail(BrixTestEntry* e, const char* msg, const char* file, int line) {
    snprintf(e->error_msg, BRIX_ERR_BUF, "%s", msg);
    snprintf(e->file, sizeof(e->file), "%s", file ? file : "");
    e->line        = line;
    g_test_failed  = 1;
    longjmp(g_test_jmp, 1);
}

// ==========================================
// Lifecycle hook registration
// ==========================================

void test_before_all_register(void* closure_ptr) {
    if (!g_suite) return;
    BrixClosure* c = (BrixClosure*)closure_ptr;
    int i = g_suite->before_all_count++;
    g_suite->before_all[i].fn_ptr  = c->fn_ptr;
    g_suite->before_all[i].env_ptr = c->env_ptr;
}

void test_after_all_register(void* closure_ptr) {
    if (!g_suite) return;
    BrixClosure* c = (BrixClosure*)closure_ptr;
    int i = g_suite->after_all_count++;
    g_suite->after_all[i].fn_ptr  = c->fn_ptr;
    g_suite->after_all[i].env_ptr = c->env_ptr;
}

void test_before_each_register(void* closure_ptr) {
    if (!g_suite) return;
    BrixClosure* c = (BrixClosure*)closure_ptr;
    int i = g_suite->before_each_count++;
    g_suite->before_each[i].fn_ptr  = c->fn_ptr;
    g_suite->before_each[i].env_ptr = c->env_ptr;
}

void test_after_each_register(void* closure_ptr) {
    if (!g_suite) return;
    BrixClosure* c = (BrixClosure*)closure_ptr;
    int i = g_suite->after_each_count++;
    g_suite->after_each[i].fn_ptr  = c->fn_ptr;
    g_suite->after_each[i].env_ptr = c->env_ptr;
}

// ==========================================
// Test registration: test.it()
// ==========================================

void test_it_register(BrixString* title, void* closure_ptr) {
    if (!g_suite) return;
    if (g_suite->test_count >= BRIX_MAX_TESTS) {
        fprintf(stderr, "Error: too many tests (max %d)\n", BRIX_MAX_TESTS);
        exit(1);
    }
    BrixClosure* c = (BrixClosure*)closure_ptr;
    int idx = g_suite->test_count++;
    BrixTestEntry* e = &g_suite->tests[idx];

    e->name = (char*)malloc(title->len + 1);
    memcpy(e->name, title->data, title->len);
    e->name[title->len] = '\0';

    e->fn_ptr       = c->fn_ptr;
    e->env_ptr      = c->env_ptr;
    e->is_async     = 0;
    e->state_ptr    = NULL;
    e->poll_fn      = NULL;
    e->passed       = 1;
    e->duration_ms  = 0;
    e->error_msg[0] = '\0';
    e->file[0]      = '\0';
    e->line         = 0;
}

// ==========================================
// Async test registration: test.it() with async closure
// ==========================================

void test_it_async(BrixString* title, void* state_ptr) {
    if (!g_suite) return;
    if (g_suite->test_count >= BRIX_MAX_TESTS) {
        fprintf(stderr, "Error: too many tests (max %d)\n", BRIX_MAX_TESTS);
        exit(1);
    }
    int idx = g_suite->test_count++;
    BrixTestEntry* e = &g_suite->tests[idx];

    e->name = (char*)malloc(title->len + 1);
    memcpy(e->name, title->data, title->len);
    e->name[title->len] = '\0';

    e->fn_ptr       = NULL;
    e->env_ptr      = NULL;
    e->is_async     = 1;
    e->state_ptr    = state_ptr;
    e->poll_fn      = (BrixPollResult (*)(void*)) ((void**)state_ptr)[0];
    e->passed       = 1;
    e->duration_ms  = 0;
    e->error_msg[0] = '\0';
    e->file[0]      = '\0';
    e->line         = 0;
}

// ==========================================
// Test runner and reporter
// ==========================================

static void brix_test_run_all(void) {
    BrixTestSuite* s = g_suite;
    double suite_start = brix_now_ms();

    // beforeAll hooks
    for (int i = 0; i < s->before_all_count; i++)
        brix_call_void(s->before_all[i].fn_ptr, s->before_all[i].env_ptr);

    for (int t = 0; t < s->test_count; t++) {
        BrixTestEntry* e = &s->tests[t];
        g_current_test_idx = t;

        // beforeEach hooks
        for (int i = 0; i < s->before_each_count; i++)
            brix_call_void(s->before_each[i].fn_ptr, s->before_each[i].env_ptr);

        double start = brix_now_ms();
        g_test_failed = 0;

        if (setjmp(g_test_jmp) == 0) {
            if (e->is_async) {
                brix_run_to_completion(e->state_ptr, e->poll_fn);
            } else {
                brix_call_void(e->fn_ptr, e->env_ptr);
            }
        }

        e->duration_ms = brix_now_ms() - start;
        e->passed = (g_test_failed == 0);

        if (e->passed) s->passed_count++;
        else           s->failed_count++;

        // afterEach hooks
        for (int i = 0; i < s->after_each_count; i++)
            brix_call_void(s->after_each[i].fn_ptr, s->after_each[i].env_ptr);
    }
    g_current_test_idx = -1;

    // afterAll hooks
    for (int i = 0; i < s->after_all_count; i++)
        brix_call_void(s->after_all[i].fn_ptr, s->after_all[i].env_ptr);

    double total_ms = brix_now_ms() - suite_start;

    // ---- Print report ----
    int all_passed = (s->failed_count == 0);
    if (all_passed)
        printf(ANSI_BOLD ANSI_GREEN "PASS" ANSI_RESET "\n");
    else
        printf(ANSI_BOLD ANSI_RED   "FAIL" ANSI_RESET "\n");

    printf("  " ANSI_BOLD "%s" ANSI_RESET "\n", s->suite_name);

    for (int t = 0; t < s->test_count; t++) {
        BrixTestEntry* e = &s->tests[t];
        if (e->passed) {
            printf(ANSI_GREEN "    ✓" ANSI_RESET " %s " ANSI_GRAY "(%.0fms)" ANSI_RESET "\n",
                   e->name, e->duration_ms);
        } else {
            printf(ANSI_RED   "    ✗" ANSI_RESET " %s " ANSI_GRAY "(%.0fms)" ANSI_RESET "\n",
                   e->name, e->duration_ms);
            if (e->error_msg[0]) {
                printf("\n%s\n\n", e->error_msg);
            }
            if (e->file[0]) {
                printf(ANSI_YELLOW "      at %s:%d" ANSI_RESET "\n\n", e->file, e->line);
            }
        }
    }

    printf("\n");
    if (all_passed)
        printf(ANSI_GREEN "Test Suites: 1 passed, 1 total" ANSI_RESET "\n");
    else
        printf(ANSI_RED   "Test Suites: 0 passed, 1 failed, 1 total" ANSI_RESET "\n");

    printf("Tests:       ");
    if (s->passed_count > 0)
        printf(ANSI_GREEN "%d passed" ANSI_RESET, s->passed_count);
    if (s->passed_count > 0 && s->failed_count > 0) printf(", ");
    if (s->failed_count > 0)
        printf(ANSI_RED   "%d failed" ANSI_RESET, s->failed_count);
    printf(", %d total\n", s->test_count);
    printf(ANSI_GRAY "Time:        %.3fs" ANSI_RESET "\n", total_ms / 1000.0);

    if (!all_passed) exit(1);
}

// ==========================================
// test.describe() entry point
// ==========================================

void test_describe_start(BrixString* title, void* closure_ptr) {
    BrixClosure* c = (BrixClosure*)closure_ptr;
    g_suite = (BrixTestSuite*)calloc(1, sizeof(BrixTestSuite));
    if (!g_suite) {
        fprintf(stderr, "Error: out of memory for test suite\n");
        exit(1);
    }
    g_suite->suite_name = (char*)malloc(title->len + 1);
    memcpy(g_suite->suite_name, title->data, title->len);
    g_suite->suite_name[title->len] = '\0';

    // Run describe closure → registers test.it() calls
    brix_call_void(c->fn_ptr, c->env_ptr);

    // Run all registered tests
    brix_test_run_all();

    // Cleanup
    for (int i = 0; i < g_suite->test_count; i++)
        if (g_suite->tests[i].name) free(g_suite->tests[i].name);
    free(g_suite->suite_name);
    free(g_suite);
    g_suite = NULL;
}

// ==========================================
// Matchers
// ==========================================

// ---- int matchers ----

void test_expect_toBe_int(long actual, long expected, char* file, int line) {
    if (actual != expected) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: %ld\n" ANSI_RESET
            "      " ANSI_RED "Received: %ld" ANSI_RESET,
            expected, actual);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_not_toBe_int(long actual, long not_expected, char* file, int line) {
    if (actual == not_expected) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: not %ld\n" ANSI_RESET
            "      " ANSI_RED "Received:     %ld" ANSI_RESET,
            not_expected, actual);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

// ---- float matchers ----

void test_expect_toBe_float(double actual, double expected, char* file, int line) {
    if (actual != expected) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: %g\n" ANSI_RESET
            "      " ANSI_RED "Received: %g" ANSI_RESET,
            expected, actual);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_not_toBe_float(double actual, double not_expected, char* file, int line) {
    if (actual == not_expected) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: not %g\n" ANSI_RESET
            "      " ANSI_RED "Received:     %g" ANSI_RESET,
            not_expected, actual);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

// ---- bool matchers ----

void test_expect_toBe_bool(long actual, long expected, char* file, int line) {
    if ((actual != 0) != (expected != 0)) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: %s\n" ANSI_RESET
            "      " ANSI_RED "Received: %s" ANSI_RESET,
            expected ? "true" : "false",
            actual   ? "true" : "false");
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

// ---- string matchers ----

void test_expect_toBe_string(BrixString* actual, BrixString* expected, char* file, int line) {
    int eq = (actual->len == expected->len) &&
             (memcmp(actual->data, expected->data, actual->len) == 0);
    if (!eq) {
        char msg[BRIX_ERR_BUF];
        int elen = (int)(expected->len < 200 ? expected->len : 200);
        int alen = (int)(actual->len   < 200 ? actual->len   : 200);
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: \"%.*s\"\n" ANSI_RESET
            "      " ANSI_RED "Received: \"%.*s\"" ANSI_RESET,
            elen, expected->data, alen, actual->data);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_not_toBe_string(BrixString* actual, BrixString* not_expected, char* file, int line) {
    int eq = (actual->len == not_expected->len) &&
             (memcmp(actual->data, not_expected->data, actual->len) == 0);
    if (eq) {
        char msg[BRIX_ERR_BUF];
        int elen = (int)(not_expected->len < 200 ? not_expected->len : 200);
        int alen = (int)(actual->len       < 200 ? actual->len       : 200);
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: not \"%.*s\"\n" ANSI_RESET
            "      " ANSI_RED "Received:     \"%.*s\"" ANSI_RESET,
            elen, not_expected->data, alen, actual->data);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

// ---- toEqual (deep equality for arrays) ----

static long brix_test_len(long rows, long cols) {
    return rows == 1 ? cols : rows;
}

static int brix_int_arrays_equal(IntMatrix* a, IntMatrix* b) {
    long total = a->rows * a->cols;
    if (total != b->rows * b->cols) return 0;
    if (brix_test_len(a->rows, a->cols) != brix_test_len(b->rows, b->cols)) return 0;
    for (long i = 0; i < total; i++) {
        if (a->data[i] != b->data[i]) return 0;
    }
    return 1;
}

static int brix_float_arrays_equal(Matrix* a, Matrix* b) {
    long total = a->rows * a->cols;
    if (total != b->rows * b->cols) return 0;
    if (brix_test_len(a->rows, a->cols) != brix_test_len(b->rows, b->cols)) return 0;
    for (long i = 0; i < total; i++) {
        if (a->data[i] != b->data[i]) return 0;
    }
    return 1;
}

static void brix_test_fail_msg(const char* msg, char* file, int line) {
    BrixTestEntry* e = brix_get_current();
    if (e) brix_test_fail(e, msg, file, line);
}

void test_expect_toEqual_int_array(IntMatrix* actual, IntMatrix* expected, char* file, int line) {
    if (!brix_int_arrays_equal(actual, expected)) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Arrays are not equal" ANSI_RESET);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_toEqual_float_array(Matrix* actual, Matrix* expected, char* file, int line) {
    if (!brix_float_arrays_equal(actual, expected)) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Arrays are not equal" ANSI_RESET);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_not_toEqual_int_array(IntMatrix* actual, IntMatrix* expected, char* file, int line) {
    if (brix_int_arrays_equal(actual, expected)) {
        brix_test_fail_msg("      " ANSI_RED "Expected arrays not to be equal" ANSI_RESET, file, line);
    }
}

void test_expect_not_toEqual_float_array(Matrix* actual, Matrix* expected, char* file, int line) {
    if (brix_float_arrays_equal(actual, expected)) {
        brix_test_fail_msg("      " ANSI_RED "Expected arrays not to be equal" ANSI_RESET, file, line);
    }
}

// ---- toBeNil ----

void test_expect_toBeNil(long is_nil_tag, char* file, int line) {
    if (!is_nil_tag) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: nil\n" ANSI_RESET
            "      " ANSI_RED "Received: <non-nil value>" ANSI_RESET);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_not_toBeNil(long is_nil_tag, char* file, int line) {
    if (is_nil_tag) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: <non-nil value>\n" ANSI_RESET
            "      " ANSI_RED "Received: nil" ANSI_RESET);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

// ---- toBeTruthy / toBeFalsy ----

void test_expect_toBeTruthy(long value, char* file, int line) {
    if (!value) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: truthy value\n" ANSI_RESET
            "      " ANSI_RED "Received: %ld (falsy)" ANSI_RESET, value);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_toBeFalsy(long value, char* file, int line) {
    if (value) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: falsy value\n" ANSI_RESET
            "      " ANSI_RED "Received: %ld (truthy)" ANSI_RESET, value);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

// ---- Numeric comparison matchers ----

// .not. comparisons fail when the positive condition holds. Not mapped to
// the opposite matcher: `not > NaN` passes, `<= NaN` would fail.
#define BRIX_NOT_CMP(name, T, fmt, op)                                          \
    void test_expect_not_##name(T actual, T threshold, char* file, int line) {  \
        if (actual op threshold) {                                              \
            char msg[BRIX_ERR_BUF];                                             \
            snprintf(msg, BRIX_ERR_BUF,                                         \
                "      " ANSI_RED "Expected: not " #op " " fmt "\n" ANSI_RESET  \
                "      " ANSI_RED "Received:   " fmt ANSI_RESET,                \
                threshold, actual);                                             \
            brix_test_fail_msg(msg, file, line);                                \
        }                                                                       \
    }
BRIX_NOT_CMP(toBeGreaterThan_int, long, "%ld", >)
BRIX_NOT_CMP(toBeGreaterThan_float, double, "%g", >)
BRIX_NOT_CMP(toBeLessThan_int, long, "%ld", <)
BRIX_NOT_CMP(toBeLessThan_float, double, "%g", <)
BRIX_NOT_CMP(toBeGreaterThanOrEqual_int, long, "%ld", >=)
BRIX_NOT_CMP(toBeGreaterThanOrEqual_float, double, "%g", >=)
BRIX_NOT_CMP(toBeLessThanOrEqual_int, long, "%ld", <=)
BRIX_NOT_CMP(toBeLessThanOrEqual_float, double, "%g", <=)

void test_expect_toBeGreaterThan_int(long actual, long threshold, char* file, int line) {
    if (actual <= threshold) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: > %ld\n" ANSI_RESET
            "      " ANSI_RED "Received:   %ld" ANSI_RESET, threshold, actual);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_toBeGreaterThan_float(double actual, double threshold, char* file, int line) {
    if (actual <= threshold) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: > %g\n" ANSI_RESET
            "      " ANSI_RED "Received:   %g" ANSI_RESET, threshold, actual);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_toBeLessThan_int(long actual, long threshold, char* file, int line) {
    if (actual >= threshold) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: < %ld\n" ANSI_RESET
            "      " ANSI_RED "Received:   %ld" ANSI_RESET, threshold, actual);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_toBeLessThan_float(double actual, double threshold, char* file, int line) {
    if (actual >= threshold) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: < %g\n" ANSI_RESET
            "      " ANSI_RED "Received:   %g" ANSI_RESET, threshold, actual);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_toBeGreaterThanOrEqual_int(long actual, long threshold, char* file, int line) {
    if (actual < threshold) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: >= %ld\n" ANSI_RESET
            "      " ANSI_RED "Received:    %ld" ANSI_RESET, threshold, actual);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_toBeGreaterThanOrEqual_float(double actual, double threshold, char* file, int line) {
    if (actual < threshold) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: >= %g\n" ANSI_RESET
            "      " ANSI_RED "Received:    %g" ANSI_RESET, threshold, actual);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_toBeLessThanOrEqual_int(long actual, long threshold, char* file, int line) {
    if (actual > threshold) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: <= %ld\n" ANSI_RESET
            "      " ANSI_RED "Received:    %ld" ANSI_RESET, threshold, actual);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_toBeLessThanOrEqual_float(double actual, double threshold, char* file, int line) {
    if (actual > threshold) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected: <= %g\n" ANSI_RESET
            "      " ANSI_RED "Received:    %g" ANSI_RESET, threshold, actual);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

// ---- toBeCloseTo (smart float precision) ----

static int brix_count_decimals(double value) {
    char buf[64];
    snprintf(buf, sizeof(buf), "%.15f", fabs(value));
    char* dot = strchr(buf, '.');
    if (!dot) return 0;
    int count = 0;
    char* p = dot + 1;
    while (*p) {
        if (*p != '0') count = (int)(p - dot);
        p++;
    }
    return count;
}

static double brix_round_to(double value, int decimals) {
    double m = pow(10.0, decimals);
    return round(value * m) / m;
}

// Equal after rounding both to the fewer decimals of the two (min 1);
// the decimals used are written to *dec.
static int brix_close_to(double actual, double expected, int* dec) {
    int dec_a = brix_count_decimals(actual);
    int dec_e = brix_count_decimals(expected);
    *dec = dec_a < dec_e ? dec_a : dec_e;
    if (*dec == 0) *dec = 1;  // epsilon minimum: at least 1 decimal
    return brix_round_to(actual, *dec) == brix_round_to(expected, *dec);
}

void test_expect_not_toBeCloseTo(double actual, double expected, char* file, int line) {
    int dec;
    if (brix_close_to(actual, expected, &dec)) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected (not close to): %.15g\n" ANSI_RESET
            "      " ANSI_RED "Received:                %.15g\n" ANSI_RESET
            "      " ANSI_GRAY "(rounded to %d decimal(s))" ANSI_RESET,
            expected, actual, dec);
        brix_test_fail_msg(msg, file, line);
    }
}

void test_expect_toBeCloseTo(double actual, double expected, char* file, int line) {
    int dec;
    if (!brix_close_to(actual, expected, &dec)) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected (close to): %.15g\n" ANSI_RESET
            "      " ANSI_RED "Received:            %.15g\n" ANSI_RESET
            "      " ANSI_GRAY "(rounded to %d decimal(s))" ANSI_RESET,
            expected, actual, dec);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

// ---- toContain (string substring) ----

// Byte-wise (memcmp), unlike brix_str_contains (strstr), so the matcher
// keeps its existing behavior on embedded NULs.
static int brix_test_str_contains(BrixString* actual, BrixString* substring) {
    if (substring->len == 0) return 1;
    if (substring->len > actual->len) return 0;
    for (long i = 0; i <= actual->len - (long)substring->len; i++) {
        if (memcmp(actual->data + i, substring->data, substring->len) == 0) return 1;
    }
    return 0;
}

static int brix_int_array_contains(IntMatrix* arr, long element) {
    long len = arr->rows * arr->cols;
    for (long i = 0; i < len; i++) {
        if (arr->data[i] == element) return 1;
    }
    return 0;
}

static int brix_float_array_contains(Matrix* arr, double element) {
    long len = arr->rows * arr->cols;
    for (long i = 0; i < len; i++) {
        if (arr->data[i] == element) return 1;
    }
    return 0;
}

void test_expect_not_toContain_string(BrixString* actual, BrixString* substring, char* file, int line) {
    if (brix_test_str_contains(actual, substring)) {
        char msg[BRIX_ERR_BUF];
        int slen = (int)(substring->len < 100 ? substring->len : 100);
        int alen = (int)(actual->len    < 100 ? actual->len    : 100);
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected string not to contain: \"%.*s\"\n" ANSI_RESET
            "      " ANSI_RED "Received:                       \"%.*s\"" ANSI_RESET,
            slen, substring->data, alen, actual->data);
        brix_test_fail_msg(msg, file, line);
    }
}

void test_expect_not_toContain_int_array(IntMatrix* arr, long element, char* file, int line) {
    if (brix_int_array_contains(arr, element)) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected array not to contain: %ld" ANSI_RESET, element);
        brix_test_fail_msg(msg, file, line);
    }
}

void test_expect_not_toContain_float_array(Matrix* arr, double element, char* file, int line) {
    if (brix_float_array_contains(arr, element)) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected array not to contain: %g" ANSI_RESET, element);
        brix_test_fail_msg(msg, file, line);
    }
}

void test_expect_toContain_string(BrixString* actual, BrixString* substring, char* file, int line) {
    if (!brix_test_str_contains(actual, substring)) {
        char msg[BRIX_ERR_BUF];
        int slen = (int)(substring->len < 100 ? substring->len : 100);
        int alen = (int)(actual->len    < 100 ? actual->len    : 100);
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected string to contain: \"%.*s\"\n" ANSI_RESET
            "      " ANSI_RED "Received:                   \"%.*s\"" ANSI_RESET,
            slen, substring->data, alen, actual->data);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

// ---- toContain (int array element) ----

void test_expect_toContain_int_array(IntMatrix* arr, long element, char* file, int line) {
    if (!brix_int_array_contains(arr, element)) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected array to contain: %ld" ANSI_RESET, element);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_toContain_float_array(Matrix* arr, double element, char* file, int line) {
    if (!brix_float_array_contains(arr, element)) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected array to contain: %g" ANSI_RESET, element);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

// ---- toHaveLength ----

void test_expect_toHaveLength_int_array(IntMatrix* arr, long expected_len, char* file, int line) {
    long actual_len = (arr->rows == 1) ? arr->cols : arr->rows;
    if (actual_len != expected_len) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected length: %ld\n" ANSI_RESET
            "      " ANSI_RED "Received length: %ld" ANSI_RESET,
            expected_len, actual_len);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_toHaveLength_float_array(Matrix* arr, long expected_len, char* file, int line) {
    long actual_len = (arr->rows == 1) ? arr->cols : arr->rows;
    if (actual_len != expected_len) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected length: %ld\n" ANSI_RESET
            "      " ANSI_RED "Received length: %ld" ANSI_RESET,
            expected_len, actual_len);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_toHaveLength_string(BrixString* s, long expected_len, char* file, int line) {
    if (s->len != expected_len) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected length: %ld\n" ANSI_RESET
            "      " ANSI_RED "Received length: %ld" ANSI_RESET,
            expected_len, s->len);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

static void brix_not_length(long actual_len, long expected_len, char* file, int line) {
    if (actual_len == expected_len) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected length: not %ld\n" ANSI_RESET
            "      " ANSI_RED "Received length:     %ld" ANSI_RESET,
            expected_len, actual_len);
        brix_test_fail_msg(msg, file, line);
    }
}

void test_expect_not_toHaveLength_int_array(IntMatrix* arr, long expected_len, char* file, int line) {
    brix_not_length(brix_test_len(arr->rows, arr->cols), expected_len, file, line);
}

void test_expect_not_toHaveLength_float_array(Matrix* arr, long expected_len, char* file, int line) {
    brix_not_length(brix_test_len(arr->rows, arr->cols), expected_len, file, line);
}

void test_expect_not_toHaveLength_string(BrixString* s, long expected_len, char* file, int line) {
    brix_not_length(s->len, expected_len, file, line);
}

// ---- toStartWith / toEndWith (v1.7 Grupo G) ----

void test_expect_toStartWith_string(BrixString* actual, BrixString* prefix, char* file, int line) {
    long ok = brix_str_starts_with(actual, prefix);
    if (!ok) {
        char msg[BRIX_ERR_BUF];
        int plen = (int)(prefix->len < 100 ? prefix->len : 100);
        int alen = (int)(actual->len < 100 ? actual->len : 100);
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected string to start with: \"%.*s\"\n" ANSI_RESET
            "      " ANSI_RED "Received:                      \"%.*s\"" ANSI_RESET,
            plen, prefix->data, alen, actual->data);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_not_toStartWith_string(BrixString* actual, BrixString* prefix, char* file, int line) {
    long ok = brix_str_starts_with(actual, prefix);
    if (ok) {
        char msg[BRIX_ERR_BUF];
        int plen = (int)(prefix->len < 100 ? prefix->len : 100);
        int alen = (int)(actual->len < 100 ? actual->len : 100);
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected string not to start with: \"%.*s\"\n" ANSI_RESET
            "      " ANSI_RED "Received:                          \"%.*s\"" ANSI_RESET,
            plen, prefix->data, alen, actual->data);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_toEndWith_string(BrixString* actual, BrixString* suffix, char* file, int line) {
    long ok = brix_str_ends_with(actual, suffix);
    if (!ok) {
        char msg[BRIX_ERR_BUF];
        int slen = (int)(suffix->len < 100 ? suffix->len : 100);
        int alen = (int)(actual->len < 100 ? actual->len : 100);
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected string to end with: \"%.*s\"\n" ANSI_RESET
            "      " ANSI_RED "Received:                    \"%.*s\"" ANSI_RESET,
            slen, suffix->data, alen, actual->data);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_not_toEndWith_string(BrixString* actual, BrixString* suffix, char* file, int line) {
    long ok = brix_str_ends_with(actual, suffix);
    if (ok) {
        char msg[BRIX_ERR_BUF];
        int slen = (int)(suffix->len < 100 ? suffix->len : 100);
        int alen = (int)(actual->len < 100 ? actual->len : 100);
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected string not to end with: \"%.*s\"\n" ANSI_RESET
            "      " ANSI_RED "Received:                        \"%.*s\"" ANSI_RESET,
            slen, suffix->data, alen, actual->data);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

// ---- toMatch (glob pattern, v1.7 Grupo G) ----
// Supports only the '*' wildcard (matches any sequence, including empty).
// '?' and other metacharacters are out of scope.

static int glob_match(const char* text, long text_len, const char* pattern, long pattern_len) {
    long ti = 0, pi = 0;
    long star_idx = -1, star_ti = -1;

    while (ti < text_len) {
        if (pi < pattern_len && pattern[pi] == '*') {
            // Record star position; tentatively match zero characters.
            star_idx = pi;
            star_ti = ti;
            pi++;
        } else if (pi < pattern_len && pattern[pi] == text[ti]) {
            pi++;
            ti++;
        } else if (star_idx != -1) {
            // Backtrack: let the last '*' consume one more character.
            pi = star_idx + 1;
            star_ti++;
            ti = star_ti;
        } else {
            return 0;
        }
    }

    // Consume any trailing '*' in the pattern.
    while (pi < pattern_len && pattern[pi] == '*') {
        pi++;
    }

    return pi == pattern_len;
}

void test_expect_matches_string(BrixString* actual, BrixString* pattern, char* file, int line) {
    int ok = glob_match(actual->data, actual->len, pattern->data, pattern->len);
    if (!ok) {
        char msg[BRIX_ERR_BUF];
        int plen = (int)(pattern->len < 100 ? pattern->len : 100);
        int alen = (int)(actual->len  < 100 ? actual->len  : 100);
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected string to match: \"%.*s\"\n" ANSI_RESET
            "      " ANSI_RED "Received:                 \"%.*s\"" ANSI_RESET,
            plen, pattern->data, alen, actual->data);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_not_matches_string(BrixString* actual, BrixString* pattern, char* file, int line) {
    int ok = glob_match(actual->data, actual->len, pattern->data, pattern->len);
    if (ok) {
        char msg[BRIX_ERR_BUF];
        int plen = (int)(pattern->len < 100 ? pattern->len : 100);
        int alen = (int)(actual->len  < 100 ? actual->len  : 100);
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected string not to match: \"%.*s\"\n" ANSI_RESET
            "      " ANSI_RED "Received:                     \"%.*s\"" ANSI_RESET,
            plen, pattern->data, alen, actual->data);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

// ---- toHaveProperty (resolved at compile-time, v1.7 Grupo G) ----

void test_expect_has_property(int has_prop, BrixString* prop_name, char* file, int line) {
    if (!has_prop) {
        char msg[BRIX_ERR_BUF];
        int nlen = (int)(prop_name->len < 100 ? prop_name->len : 100);
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected struct to have property: \"%.*s\"" ANSI_RESET,
            nlen, prop_name->data);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_not_has_property(int has_prop, BrixString* prop_name, char* file, int line) {
    if (has_prop) {
        char msg[BRIX_ERR_BUF];
        int nlen = (int)(prop_name->len < 100 ? prop_name->len : 100);
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected struct not to have property: \"%.*s\"" ANSI_RESET,
            nlen, prop_name->data);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

// ---- toThrow (v1.7 Grupo H) ----
// The fork() + closure-call dance happens in codegen (LLVM IR), because each
// Brix closure has its own signature. This helper only decodes the child's
// exit status once codegen has forked, run the closure, and waited on it.

// brix_wait_for_child: waits on `pid` and reports whether it "threw"
// (non-zero exit code or killed by signal). Returns 1 if it threw, 0 otherwise.
int brix_wait_for_child(int pid) {
    int status = 0;
    if (waitpid(pid, &status, 0) < 0) {
        return 1;
    }
    if (WIFEXITED(status) && WEXITSTATUS(status) != 0) {
        return 1;
    }
    if (WIFSIGNALED(status)) {
        return 1;
    }
    return 0;
}

void test_expect_to_throw(int threw, char* file, int line) {
    if (!threw) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected function to throw" ANSI_RESET);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_not_to_throw(int threw, char* file, int line) {
    if (threw) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected function not to throw" ANSI_RESET);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

// ==========================================
// IN-PLACE MUTABLE ARRAY OPERATIONS (v1.9 Grupo E)
// ==========================================

// --- IntMatrix In-Place Methods ---

void intmatrix_push_inplace(IntMatrix** m_ptr, long val) {
    if (!m_ptr || !*m_ptr) {
        fprintf(stderr, "Runtime Error: push! called on NULL IntMatrix\n");
        exit(1);
    }
    IntMatrix* m = *m_ptr;
    if (m->rows != 1) {
        fprintf(stderr, "Runtime Error: push! is only supported on 1D arrays (rows == 1)\n");
        exit(1);
    }
    long old_cols = m->cols;
    long new_cols = old_cols + 1;
    m->data = (long*)realloc(m->data, new_cols * sizeof(long));
    m->data[old_cols] = val;
    m->cols = new_cols;
}

long intmatrix_pop_inplace(IntMatrix* m) {
    if (!m) {
        fprintf(stderr, "Runtime Error: pop! called on NULL IntMatrix\n");
        exit(1);
    }
    if (m->rows != 1) {
        fprintf(stderr, "Runtime Error: pop! is only supported on 1D arrays (rows == 1)\n");
        exit(1);
    }
    if (m->cols == 0) {
        fprintf(stderr, "Runtime Error: pop! on empty array\n");
        exit(1);
    }
    long val = m->data[m->cols - 1];
    m->cols -= 1;
    return val;
}

void intmatrix_insert_inplace(IntMatrix** m_ptr, long idx, long val) {
    if (!m_ptr || !*m_ptr) {
        fprintf(stderr, "Runtime Error: insert! called on NULL IntMatrix\n");
        exit(1);
    }
    IntMatrix* m = *m_ptr;
    if (m->rows != 1) {
        fprintf(stderr, "Runtime Error: insert! is only supported on 1D arrays (rows == 1)\n");
        exit(1);
    }
    if (idx < 0 || idx > m->cols) {
        fprintf(stderr, "Runtime Error: insert! index out of bounds (index %ld, size %ld)\n", idx, m->cols);
        exit(1);
    }
    long old_cols = m->cols;
    long new_cols = old_cols + 1;
    m->data = (long*)realloc(m->data, new_cols * sizeof(long));
    for (long i = old_cols; i > idx; i--) {
        m->data[i] = m->data[i - 1];
    }
    m->data[idx] = val;
    m->cols = new_cols;
}

long intmatrix_remove_inplace(IntMatrix* m, long idx) {
    if (!m) {
        fprintf(stderr, "Runtime Error: remove! called on NULL IntMatrix\n");
        exit(1);
    }
    if (m->rows != 1) {
        fprintf(stderr, "Runtime Error: remove! is only supported on 1D arrays (rows == 1)\n");
        exit(1);
    }
    if (idx < 0 || idx >= m->cols) {
        fprintf(stderr, "Runtime Error: remove! index out of bounds (index %ld, size %ld)\n", idx, m->cols);
        exit(1);
    }
    long removed = m->data[idx];
    for (long i = idx; i < m->cols - 1; i++) {
        m->data[i] = m->data[i + 1];
    }
    m->cols -= 1;
    return removed;
}

void intmatrix_sort_inplace(IntMatrix* m) {
    if (!m) {
        fprintf(stderr, "Runtime Error: sort! called on NULL IntMatrix\n");
        exit(1);
    }
    if (m->rows != 1) {
        fprintf(stderr, "Runtime Error: sort! is only supported on 1D arrays (rows == 1)\n");
        exit(1);
    }
    qsort(m->data, m->cols, sizeof(long), compare_longs);
}

void intmatrix_reverse_inplace(IntMatrix* m) {
    if (!m) {
        fprintf(stderr, "Runtime Error: reverse! called on NULL IntMatrix\n");
        exit(1);
    }
    if (m->rows != 1) {
        fprintf(stderr, "Runtime Error: reverse! is only supported on 1D arrays (rows == 1)\n");
        exit(1);
    }
    long i = 0, j = m->cols - 1;
    while (i < j) {
        long tmp = m->data[i];
        m->data[i] = m->data[j];
        m->data[j] = tmp;
        i++;
        j--;
    }
}

void intmatrix_clear_inplace(IntMatrix* m) {
    if (!m) {
        fprintf(stderr, "Runtime Error: clear! called on NULL IntMatrix\n");
        exit(1);
    }
    if (m->rows != 1) {
        fprintf(stderr, "Runtime Error: clear! is only supported on 1D arrays (rows == 1)\n");
        exit(1);
    }
    m->cols = 0;
}

// --- Matrix (float) In-Place Methods ---

void matrix_push_inplace(Matrix** m_ptr, double val) {
    if (!m_ptr || !*m_ptr) {
        fprintf(stderr, "Runtime Error: push! called on NULL Matrix\n");
        exit(1);
    }
    Matrix* m = *m_ptr;
    if (m->rows != 1) {
        fprintf(stderr, "Runtime Error: push! is only supported on 1D arrays (rows == 1)\n");
        exit(1);
    }
    long old_cols = m->cols;
    long new_cols = old_cols + 1;
    m->data = (double*)realloc(m->data, new_cols * sizeof(double));
    m->data[old_cols] = val;
    m->cols = new_cols;
}

double matrix_pop_inplace(Matrix* m) {
    if (!m) {
        fprintf(stderr, "Runtime Error: pop! called on NULL Matrix\n");
        exit(1);
    }
    if (m->rows != 1) {
        fprintf(stderr, "Runtime Error: pop! is only supported on 1D arrays (rows == 1)\n");
        exit(1);
    }
    if (m->cols == 0) {
        fprintf(stderr, "Runtime Error: pop! on empty array\n");
        exit(1);
    }
    double val = m->data[m->cols - 1];
    m->cols -= 1;
    return val;
}

void matrix_insert_inplace(Matrix** m_ptr, long idx, double val) {
    if (!m_ptr || !*m_ptr) {
        fprintf(stderr, "Runtime Error: insert! called on NULL Matrix\n");
        exit(1);
    }
    Matrix* m = *m_ptr;
    if (m->rows != 1) {
        fprintf(stderr, "Runtime Error: insert! is only supported on 1D arrays (rows == 1)\n");
        exit(1);
    }
    if (idx < 0 || idx > m->cols) {
        fprintf(stderr, "Runtime Error: insert! index out of bounds (index %ld, size %ld)\n", idx, m->cols);
        exit(1);
    }
    long old_cols = m->cols;
    long new_cols = old_cols + 1;
    m->data = (double*)realloc(m->data, new_cols * sizeof(double));
    for (long i = old_cols; i > idx; i--) {
        m->data[i] = m->data[i - 1];
    }
    m->data[idx] = val;
    m->cols = new_cols;
}

double matrix_remove_inplace(Matrix* m, long idx) {
    if (!m) {
        fprintf(stderr, "Runtime Error: remove! called on NULL Matrix\n");
        exit(1);
    }
    if (m->rows != 1) {
        fprintf(stderr, "Runtime Error: remove! is only supported on 1D arrays (rows == 1)\n");
        exit(1);
    }
    if (idx < 0 || idx >= m->cols) {
        fprintf(stderr, "Runtime Error: remove! index out of bounds (index %ld, size %ld)\n", idx, m->cols);
        exit(1);
    }
    double removed = m->data[idx];
    for (long i = idx; i < m->cols - 1; i++) {
        m->data[i] = m->data[i + 1];
    }
    m->cols -= 1;
    return removed;
}

void matrix_sort_inplace(Matrix* m) {
    if (!m) {
        fprintf(stderr, "Runtime Error: sort! called on NULL Matrix\n");
        exit(1);
    }
    if (m->rows != 1) {
        fprintf(stderr, "Runtime Error: sort! is only supported on 1D arrays (rows == 1)\n");
        exit(1);
    }
    qsort(m->data, m->cols, sizeof(double), compare_doubles);
}

void matrix_reverse_inplace(Matrix* m) {
    if (!m) {
        fprintf(stderr, "Runtime Error: reverse! called on NULL Matrix\n");
        exit(1);
    }
    if (m->rows != 1) {
        fprintf(stderr, "Runtime Error: reverse! is only supported on 1D arrays (rows == 1)\n");
        exit(1);
    }
    long i = 0, j = m->cols - 1;
    while (i < j) {
        double tmp = m->data[i];
        m->data[i] = m->data[j];
        m->data[j] = tmp;
        i++;
        j--;
    }
}

void matrix_clear_inplace(Matrix* m) {
    if (!m) {
        fprintf(stderr, "Runtime Error: clear! called on NULL Matrix\n");
        exit(1);
    }
    if (m->rows != 1) {
        fprintf(stderr, "Runtime Error: clear! is only supported on 1D arrays (rows == 1)\n");
        exit(1);
    }
    m->cols = 0;
}
