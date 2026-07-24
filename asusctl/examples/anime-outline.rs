use rog_anime::usb::get_anime_type;
use rog_anime::AnimeDataBuffer;
use rog_dbus::zbus_anime::AnimeProxyBlocking;
use zbus::blocking::Connection;

// In usable data:
// Top row start at 1, ends at 32

fn main() {
    let conn = Connection::system().unwrap();
    let proxy = AnimeProxyBlocking::new(&conn).unwrap();
    let anime_type = get_anime_type();
    let mut matrix = AnimeDataBuffer::new(anime_type);
    matrix.data_mut()[1] = 100; // start = 1
    for n in matrix.data_mut()[2..32].iter_mut() {
        *n = 250;
    }
    matrix.data_mut()[32] = 100; // end
    matrix.data_mut()[34] = 100; // start x = 0
    matrix.data_mut()[66] = 100; // end
    matrix.data_mut()[69] = 100; // start x = 1
    matrix.data_mut()[101] = 100; // end
    matrix.data_mut()[102] = 100; // start
    matrix.data_mut()[134] = 100; // end
    matrix.data_mut()[137] = 100; // start
    matrix.data_mut()[169] = 100; // end
    matrix.data_mut()[170] = 100; // start
    matrix.data_mut()[202] = 100; // end
    matrix.data_mut()[204] = 100; // start
    matrix.data_mut()[236] = 100; // end
    matrix.data_mut()[237] = 100; // start
    matrix.data_mut()[268] = 100; // end
    matrix.data_mut()[270] = 100; // start
    matrix.data_mut()[301] = 100; // end
    matrix.data_mut()[302] = 100; // start
    matrix.data_mut()[332] = 100; // end
    matrix.data_mut()[334] = 100; // start
    matrix.data_mut()[364] = 100; // end
    matrix.data_mut()[365] = 100; // start
    matrix.data_mut()[394] = 100; // end
    matrix.data_mut()[396] = 100; // start
    matrix.data_mut()[425] = 100; // end
    matrix.data_mut()[426] = 100; // start
    matrix.data_mut()[454] = 100; // end
    matrix.data_mut()[456] = 100; // start
    matrix.data_mut()[484] = 100; // end
    matrix.data_mut()[485] = 100; // start
    matrix.data_mut()[512] = 100; // end
    matrix.data_mut()[514] = 100; // start
    matrix.data_mut()[541] = 100; // end
    matrix.data_mut()[542] = 100; // start
    matrix.data_mut()[568] = 100; // end
    matrix.data_mut()[570] = 100; // start
    matrix.data_mut()[596] = 100; // end
    matrix.data_mut()[597] = 100; // start
    matrix.data_mut()[622] = 100; // end
    matrix.data_mut()[624] = 100; // start
    matrix.data_mut()[649] = 100; // end
    matrix.data_mut()[650] = 100; // start
    matrix.data_mut()[674] = 100; // end
    matrix.data_mut()[676] = 100; // start
    matrix.data_mut()[700] = 100; // end
    matrix.data_mut()[701] = 100; // start
    matrix.data_mut()[724] = 100; // end
    matrix.data_mut()[726] = 100; // start
    matrix.data_mut()[749] = 100; // end
    matrix.data_mut()[750] = 100; // start
    matrix.data_mut()[772] = 100; // end
    matrix.data_mut()[774] = 100; // start
    matrix.data_mut()[796] = 100; // end
    matrix.data_mut()[797] = 100; // start
    matrix.data_mut()[818] = 100; // end
    matrix.data_mut()[820] = 100; // start
    matrix.data_mut()[841] = 100; // end
    matrix.data_mut()[842] = 100; // start
    matrix.data_mut()[862] = 100; // end
    matrix.data_mut()[864] = 100; // start
    matrix.data_mut()[884] = 100; // end
    matrix.data_mut()[885] = 100; // start
    matrix.data_mut()[904] = 100; // end
    matrix.data_mut()[906] = 100; // start
    matrix.data_mut()[925] = 100; // end
    matrix.data_mut()[926] = 100; // start
    matrix.data_mut()[944] = 100; // end
    matrix.data_mut()[946] = 100; // start
    matrix.data_mut()[964] = 100; // end
    matrix.data_mut()[965] = 100; // start
    matrix.data_mut()[982] = 100; // end
    matrix.data_mut()[984] = 100; // start
    matrix.data_mut()[1001] = 100; // end
    matrix.data_mut()[1002] = 100; // start
    matrix.data_mut()[1018] = 100; // end
    matrix.data_mut()[1020] = 100; // start
    matrix.data_mut()[1036] = 100; // end
    matrix.data_mut()[1037] = 100; // start
    matrix.data_mut()[1052] = 100; // end
    matrix.data_mut()[1054] = 100; // start
    matrix.data_mut()[1069] = 100; // end
    matrix.data_mut()[1070] = 100; // start
    matrix.data_mut()[1084] = 100; // end
    matrix.data_mut()[1086] = 100; // start
    matrix.data_mut()[1100] = 100; // end
    matrix.data_mut()[1101] = 100; // start
    matrix.data_mut()[1114] = 100; // end
    matrix.data_mut()[1116] = 100; // start
    matrix.data_mut()[1129] = 100; // end
    matrix.data_mut()[1130] = 100; // start
    matrix.data_mut()[1142] = 100; // end
    matrix.data_mut()[1144] = 100; // start
    matrix.data_mut()[1156] = 100; // end
    matrix.data_mut()[1157] = 100; // start
    matrix.data_mut()[1168] = 100; // end
    matrix.data_mut()[1170] = 100; // start
    matrix.data_mut()[1181] = 100; // end
    matrix.data_mut()[1182] = 100; // start
    matrix.data_mut()[1192] = 100; // end
    matrix.data_mut()[1194] = 100; // start
    matrix.data_mut()[1204] = 100; // end
    matrix.data_mut()[1205] = 100; // start
    matrix.data_mut()[1214] = 100; // end
    matrix.data_mut()[1216] = 100; // start
    matrix.data_mut()[1225] = 100; // end
    matrix.data_mut()[1226] = 100; // start
    matrix.data_mut()[1234] = 100; // end
    matrix.data_mut()[1236] = 100; // start
    for n in matrix.data_mut()[1237..1244].iter_mut() {
        *n = 250;
    }
    matrix.data_mut()[1244] = 100; // end
    println!("{:?}", matrix);

    proxy.write(matrix).unwrap();
}
