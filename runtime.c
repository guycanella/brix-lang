#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <time.h>
#include <setjmp.h>

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
  m->ref_count = 1;  // Initialize ARC
  m->rows = rows;
  m->cols = cols;
  m->data = (double *)malloc(rows * cols * sizeof(double));
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
  m->ref_count = 1;  // Initialize ARC
  m->rows = rows;
  m->cols = cols;
  m->data = (long *)calloc(rows * cols, sizeof(long));  // calloc zeros memory
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
            brix_call_void(e->fn_ptr, e->env_ptr);
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

void test_expect_toEqual_int_array(IntMatrix* actual, IntMatrix* expected, char* file, int line) {
    long alen = (actual->rows == 1)   ? actual->cols   : actual->rows;
    long elen = (expected->rows == 1) ? expected->cols : expected->rows;
    int eq = (alen == elen);
    if (eq) {
        for (long i = 0; i < alen; i++) {
            if (actual->data[i] != expected->data[i]) { eq = 0; break; }
        }
    }
    if (!eq) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Arrays are not equal" ANSI_RESET);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_toEqual_float_array(Matrix* actual, Matrix* expected, char* file, int line) {
    long alen = (actual->rows == 1)   ? actual->cols   : actual->rows;
    long elen = (expected->rows == 1) ? expected->cols : expected->rows;
    int eq = (alen == elen);
    if (eq) {
        for (long i = 0; i < alen; i++) {
            if (actual->data[i] != expected->data[i]) { eq = 0; break; }
        }
    }
    if (!eq) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Arrays are not equal" ANSI_RESET);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
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

void test_expect_toBeCloseTo(double actual, double expected, char* file, int line) {
    int dec_a = brix_count_decimals(actual);
    int dec_e = brix_count_decimals(expected);
    int dec   = dec_a < dec_e ? dec_a : dec_e;
    if (dec == 0) dec = 1;  // epsilon minimum: at least 1 decimal

    double ra = brix_round_to(actual,   dec);
    double re = brix_round_to(expected, dec);

    if (ra != re) {
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

void test_expect_toContain_string(BrixString* actual, BrixString* substring, char* file, int line) {
    int found = 0;
    if (substring->len == 0) { found = 1; }
    else if (substring->len <= actual->len) {
        for (long i = 0; i <= actual->len - (long)substring->len; i++) {
            if (memcmp(actual->data + i, substring->data, substring->len) == 0) {
                found = 1; break;
            }
        }
    }
    if (!found) {
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
    long len = (arr->rows == 1) ? arr->cols : arr->rows;
    int found = 0;
    for (long i = 0; i < len; i++) {
        if (arr->data[i] == element) { found = 1; break; }
    }
    if (!found) {
        char msg[BRIX_ERR_BUF];
        snprintf(msg, BRIX_ERR_BUF,
            "      " ANSI_RED "Expected array to contain: %ld" ANSI_RESET, element);
        BrixTestEntry* e = brix_get_current();
        if (e) brix_test_fail(e, msg, file, line);
    }
}

void test_expect_toContain_float_array(Matrix* arr, double element, char* file, int line) {
    long len = (arr->rows == 1) ? arr->cols : arr->rows;
    int found = 0;
    for (long i = 0; i < len; i++) {
        if (arr->data[i] == element) { found = 1; break; }
    }
    if (!found) {
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