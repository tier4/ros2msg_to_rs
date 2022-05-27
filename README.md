# Rust code generator from .msg and .srv of ROS2

This is used for [safe_drive](https://github.com/tier4/safe_drive), a Rust bindings of ROS2.

## Types (Galactic)

| ROS           | C                                 | Rust      |
|---------------|-----------------------------------|-----------|
| bool          | bool                              | bool      |
| int8          | int8_t                            | i8        |
| uint8         | uint8_t                           | u8        |
| int16         | int16_t                           | i16       |
| uint16        | uint16_t                          | u16       |
| int32         | int32_t                           | i32       |
| uint32        | uint32_t                          | u32       |
| int64         | int64_t                           | i64       |
| uint64        | uint64_t                          | u64       |
| float32       | float                             | f32       |
| float64       | double                            | f64       |
| string        | rosidl_runtime_c__String          |           |
| int32[]       | rosidl_runtime_c__int32__Sequence |           |
| int32[10]     | int32_t var[10]                   | [i32; 10] |

[rosidl_runtime_c__String](https://docs.ros2.org/galactic/api/rosidl_runtime_c/structrosidl__runtime__c____String.html)

```c
struct rosidl_runtime_c__String {
    char *data;
    size_t size;
    size_t capacity;
}
```

[rosidl_runtime_c__int32__Sequence](https://docs.ros2.org/galactic/api/rosidl_runtime_c/primitives__sequence_8h_source.html)

```c
#define ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(STRUCT_NAME, TYPE_NAME) \
typedef struct rosidl_runtime_c__ ## STRUCT_NAME ## __Sequence \
{ \
    TYPE_NAME * data;  \
    size_t size;  \
    size_t capacity;  \
} rosidl_runtime_c__ ## STRUCT_NAME ## __Sequence;

// sequence types for all basic types
ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(float, float)
ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(double, double)
ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(long_double, long double)
ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(char, signed char)
ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(wchar, uint16_t)
ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(boolean, bool)
ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(octet, uint8_t)
ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(uint8, uint8_t)
ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(int8, int8_t)
ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(uint16, uint16_t)
ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(int16, int16_t)
ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(uint32, uint32_t)
ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(int32, int32_t)
ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(uint64, uint64_t)
ROSIDL_RUNTIME_C__PRIMITIVE_SEQUENCE(int64, int64_t)
```
