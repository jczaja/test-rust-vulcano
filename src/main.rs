use vulkano::VulkanLibrary;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::device::{physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, QueueCreateInfo, QueueFlags, };
use vulkano::device::Properties;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::memory::allocator::{AllocationCreateInfo,MemoryUsage};
use vulkano::command_buffer::allocator::{ StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo, };
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferInfo};
use vulkano::sync::{self, GpuFuture,PipelineStage};
use vulkano::pipeline::ComputePipeline;

use vulkano::pipeline::Pipeline;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;

use vulkano::pipeline::PipelineBindPoint;

use vulkano::query::{QueryControlFlags,QueryPool, QueryPoolCreateInfo,QueryResultFlags, QueryType};

// TODO: inline assembly spirv
// TODO: check caps int8, float, double

fn main() {
    println!("Hello, world!");

    let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
    let instance = Instance::new(library, InstanceCreateInfo::default())
        .expect("failed to create instance");

    // Pick the first found physical device capable of Vulkan (my integrated GPU)
    let physical_device = instance
        .enumerate_physical_devices()
        .expect("could not enumerate devices")
        .next()
        .expect("no devices available");


    for family in physical_device.queue_family_properties() {
        println!("Found a queue family with {:?} queue(s)", family.queue_count);
    }

    let queue_family_index = physical_device
        .queue_family_properties()
        .iter()
        .enumerate()
        .position(|(_queue_family_index, queue_family_properties)| {
            queue_family_properties.queue_flags.contains(QueueFlags::COMPUTE)
        })
        .expect("couldn't find a graphical queue family") as u32;


    let (device, mut queues) = Device::new(
        physical_device,
        DeviceCreateInfo {
            // here we pass the desired queue family to use by index
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            ..Default::default()
        },
    )
    .expect("failed to create device");
    // lets get first queue
    let queue = queues.next().expect("Error getting first queue");

    // Allocate buffer on GPU
    let memory_allocator = StandardMemoryAllocator::new_default(device.clone());

    const MAX_IDX : u32 = 512*48*96*8; 
                          
    let data_iter = 0..MAX_IDX;
    let data_buffer = Buffer::from_iter(
        &memory_allocator,
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            usage: MemoryUsage::Upload,
            ..Default::default()
        },
        data_iter,
    )
    .expect("failed to create buffer");

    const COMPUTE_SHADER: &[u8] = include_bytes!(env!("shader.spv"));

     let shader = unsafe {
            vulkano::shader::ShaderModule::from_bytes(device.clone(), COMPUTE_SHADER)
                .unwrap()
        };

    let compute_pipeline = ComputePipeline::new(
        device.clone(),
        shader.entry_point("main_cs").unwrap(),
        &(),
        None,
        |_| {},
    )
    .expect("failed to create compute pipeline");

    let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone());
    let pipeline_layout = compute_pipeline.layout();
    let descriptor_set_layouts = pipeline_layout.set_layouts();

    let descriptor_set_layout_index = 0;
    let descriptor_set_layout = descriptor_set_layouts
        .get(descriptor_set_layout_index)
        .unwrap();
    let descriptor_set = PersistentDescriptorSet::new(
        &descriptor_set_allocator,
        descriptor_set_layout.clone(),
        [WriteDescriptorSet::buffer(0, data_buffer.clone())], // 0 is the binding
    )
    .unwrap();

    // Timestamp query to get measure of GPU execution of kernel
    let qi = QueryPoolCreateInfo { query_count: 2, ..QueryPoolCreateInfo::query_type(QueryType::Timestamp) };
    let query_pool = QueryPool::new(device.clone(),qi).expect("Error: Unable to create query pool");
    let mut query_results = [0u64;2];

    let command_buffer_allocator = StandardCommandBufferAllocator::new(
        device.clone(),
        StandardCommandBufferAllocatorCreateInfo::default(),
    );

    let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
        &command_buffer_allocator,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    let timestamp_period = device.physical_device().properties().timestamp_period;

    // reset query pool
    let work_group_counts = [MAX_IDX/(768), 1, 1];
    unsafe {
        command_buffer_builder.reset_query_pool(query_pool.clone(),0..2).expect("Error: unable to reset query pool");


    command_buffer_builder
    .bind_pipeline_compute(compute_pipeline.clone())
    .write_timestamp(query_pool.clone(),0,PipelineStage::TopOfPipe).expect("Error setting first write timestamp on TOP OF PIPE")
    .bind_descriptor_sets(
        PipelineBindPoint::Compute,
        compute_pipeline.layout().clone(),
        descriptor_set_layout_index as u32,
        descriptor_set,
    )
    .write_timestamp(query_pool.clone(),1,PipelineStage::BottomOfPipe).expect("Error setting second write timestamp on BOTTOM OF PIPE")
    .dispatch(work_group_counts)
    .unwrap();
    }

    let command_buffer = command_buffer_builder.build().unwrap();

    // Send commands (copying) to GPU
    let future = sync::now(device.clone())
    .then_execute(queue.clone(), command_buffer)
    .unwrap()
    .then_signal_fence_and_flush() // same as signal fence, and then flush
    .unwrap();

    future.wait(None).unwrap();  // None is an optional timeout

    println!("PRE TIMESTAMPS Start: {}, End: {}, TimeStampPeriod: {}",query_results[0],query_results[1],timestamp_period);
    query_pool.queries_range(0..2)
                .unwrap()
                .get_results(
                    &mut query_results,
                    QueryResultFlags::WAIT


                )
                .expect("Error getting queries");


    println!("TIMESTAMPS Start: {}, End: {}, DIFF[ms]: {}",query_results[0],query_results[1],(timestamp_period as f64 * (query_results[1]-query_results[0]) as f64)/1000000.0);
    
    println!("Prime numbers:");
//    let content = data_buffer.read().unwrap();
//    for val in content.iter() {
//        if *val != 1 {
//            println!("{}",val);
//        }
//    }
}



