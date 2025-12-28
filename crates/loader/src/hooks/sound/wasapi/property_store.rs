use windows::Win32::{
    Devices::FunctionDiscovery::{
        PKEY_Device_ContainerId,
        PKEY_Device_DeviceDesc,
        PKEY_Device_FriendlyName,
        PKEY_Device_InstanceId,
        PKEY_DeviceInterface_FriendlyName,
    },
    Foundation::{
        ERROR_NOT_FOUND,
        PROPERTYKEY,
        STG_E_ACCESSDENIED,
    },
    System::Com::StructuredStorage::PROPVARIANT,
    UI::Shell::PropertiesSystem::{
        IPropertyStore,
        IPropertyStore_Impl,
    },
};
use windows_core::implement;

#[implement(IPropertyStore)]
pub(in crate::hooks::sound) struct MyDevicePropertyStore {
    items: [(PROPERTYKEY, PROPVARIANT); 5],
}

impl MyDevicePropertyStore {
    pub fn new() -> Self {
        let k_i_f_name = PKEY_DeviceInterface_FriendlyName;
        let k_dev_desc = PKEY_Device_DeviceDesc;
        let k_f_name = PKEY_Device_FriendlyName;
        let k_inst_id = PKEY_Device_InstanceId;
        let k_c_id = PKEY_Device_ContainerId;
        let v_i_f_name = PROPVARIANT::from("114");
        let v_dev_desc = PROPVARIANT::from("514");
        let v_f_name = PROPVARIANT::from("1919");
        let v_inst_id = PROPVARIANT::from("810");
        let v_c_id = PROPVARIANT::from("1919810");
        let items = [
            (k_i_f_name, v_i_f_name),
            (k_dev_desc, v_dev_desc),
            (k_f_name, v_f_name),
            (k_inst_id, v_inst_id),
            (k_c_id, v_c_id),
        ];
        Self { items }
    }
}

#[allow(non_snake_case)]
impl IPropertyStore_Impl for MyDevicePropertyStore_Impl {
    fn GetCount(&self) -> windows_result::Result<u32> {
        Ok(self.items.len() as u32)
    }

    fn GetAt(&self, i: u32, out_key: *mut PROPERTYKEY) -> windows_result::Result<()> {
        let k = self.items.get(i as usize).ok_or(ERROR_NOT_FOUND)?;
        unsafe {
            *out_key = k.0;
        }
        Ok(())
    }

    fn GetValue(&self, key: *const PROPERTYKEY) -> windows_result::Result<PROPVARIANT> {
        let k = unsafe { *key };
        for i in 0..self.items.len() {
            if self.items[i].0 == k {
                return Ok(self.items[i].1.clone());
            }
        }
        Err(ERROR_NOT_FOUND)?
    }

    fn SetValue(
        &self,
        _key: *const PROPERTYKEY,
        _prop_var: *const PROPVARIANT,
    ) -> windows_result::Result<()> {
        Err(STG_E_ACCESSDENIED)?
    }

    fn Commit(&self) -> windows_result::Result<()> {
        Err(STG_E_ACCESSDENIED)?
    }
}
