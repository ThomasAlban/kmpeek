use bevy::math::Vec3;
use bytemuck::{cast_slice_mut, Pod};
use byteorder::{ReadBytesExt, WriteBytesExt, BE};
use num_traits::PrimInt;
use std::io::{self, Read, Write};

// add extra functions to the Read and Write traits to make it easier to read and write vec3s and arrays
pub trait ReadArrays: Read {
    fn read_vec3(&mut self) -> io::Result<Vec3> {
        let mut result = [0f32; 3];
        for x in result.iter_mut() {
            *x = self.read_f32::<BE>()?;
        }
        Ok(result.into())
    }
    // takes in T (the numeric type of the array) and N (number of elements) and reads an array of that size and type
    fn read_array<T, const N: usize>(&mut self) -> io::Result<[T; N]>
    where
        T: Default + Pod + PrimInt,
    {
        // create an array of the default value of T (will be zeros) with length N
        let mut result = [T::default(); N];
        // read the exact number of bytes required to fill result
        self.read_exact(cast_slice_mut(&mut result))?;
        // convert each element of result to big endian
        result.iter_mut().for_each(|x| *x = x.to_be());
        Ok(result)
    }
}
impl<R: Read> ReadArrays for R {}
pub trait WriteArrays: Write {
    fn write_vec3(&mut self, vec3: Vec3) -> io::Result<()> {
        self.write_f32::<BE>(vec3.x)?;
        self.write_f32::<BE>(vec3.y)?;
        self.write_f32::<BE>(vec3.z)?;
        Ok(())
    }
    fn write_array<T, const N: usize>(&mut self, mut array: [T; N]) -> io::Result<()>
    where
        T: Pod + PrimInt,
    {
        array.iter_mut().for_each(|x| *x = x.to_be());
        let result: &[u8] = cast_slice_mut(&mut array);
        self.write_all(result)?;
        Ok(())
    }
}
impl<W: Write> WriteArrays for W {}
