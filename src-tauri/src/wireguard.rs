//use std::str::FromStr;

//use wireguard_rs::{
//netlink::{address_interface, create_interface},
//wgapi::WGApi,
//IpAddrMask,
//};

//use crate::{database::Location, error::Error};

/*/// Setup client interface*/
/*pub fn setup_interface(location: Location) -> Result<(), Error> {*/
/*create_interface(&location.name)?;*/
/*address_interface(&location.name, &IpAddrMask::from_str(&location.address)?)?;*/
/*let api = WGApi::new(location.name, false);*/
/*let mut host = api.read_host()?;*/

/*Ok(())*/
/*}*/
