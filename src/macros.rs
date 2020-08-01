#[macro_export]
macro_rules! map {
	() => ({
		let temp_map = ::std::collections::HashMap::new();
		temp_map
	});
	($($key:expr => $value:expr),+) => ({
		let mut temp_map = ::std::collections::HashMap::new();
		$(
			temp_map.insert($key, $value);
		)+
		temp_map
	});
}

#[macro_export]
macro_rules! set {
	() => ({
		let temp_set = ::std::collections::HashSet::new();
		temp_set
	});
	($($x:expr),+) => ({ // Match one or more comma delimited items
		let mut temp_set = ::std::collections::HashSet::new();  // Create a mutable HashSet
		$(
			temp_set.insert($x); // Insert each item matched into the HashSet
		)+
		temp_set // Return the populated HashSet
	});
}
